//! Renders a 2D scene containing a single, moving sprite.
use std::collections::HashMap;

// use std::sync::mpsc::{channel, Receiver, Sender};
// use async_std::channel::{unbounded, Receiver, Sender};
use crossbeam::channel::{unbounded, Receiver, Sender};

use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use wasm_bindgen::prelude::*;

mod web_communication {
    use std::marker::PhantomData;

    // use std::sync::mpsc::{channel as unbounded, Receiver, Sender};
    // use async_std::channel::{unbounded, Receiver, Sender};
    use crossbeam::channel::{unbounded, Receiver, Sender};

    use bevy::{ecs::event::Event, prelude::*};
    use once_cell::sync::OnceCell;
    use serde::{de::DeserializeOwned, Serialize};
    use wasm_bindgen::prelude::*;
    use web_sys::console;

    /// Wrapper around Receiver, just to derive [`Resource`].
    #[derive(Resource)]
    pub struct ReceiverResource<T> {
        rx: Receiver<T>,
    }

    /// system that sends postMessage to the parent window
    pub fn postmessage_to_parent<OUTPUT: std::fmt::Debug + Send + Sync + Event + Serialize>(
        mut postevt: EventReader<OUTPUT>,
    ) {
        for evt in postevt.iter() {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(parent)) = window.top() {
                    // gloo::console::log!(format!("{evt:?}"));
                    if parent != window {
                        parent
                            .post_message(&serde_wasm_bindgen::to_value(evt).unwrap(), "*")
                            .ok();
                    }
                } else {
                    // window.post_message(&serde_wasm_bindgen::to_value(evt).unwrap(), "*");
                }
            }
        }
    }
    pub fn postmessage_input<INPUT: Send + Sync + Event>(
        rx: Res<ReceiverResource<INPUT>>,
        mut evt: EventWriter<INPUT>,
    ) {
        if let Ok(rx) = rx.rx.try_recv() {
            evt.send(rx);
        }
    }

    // #[wasm_bindgen]
    // pub fn js_event(val: JsValue) {
    //     match serde_wasm_bindgen::from_value(val) {
    //         Ok(cmd) => {
    //             if let Some(sender) = SENDER.get() {
    //                 sender.send(cmd);
    //                 // sender.send(cmd).await;
    //             }
    //         }
    //         Err(e) => (console::log_1(&"fail to deserialize".into())),
    //     }
    // }

    pub struct WebPostMessage<INPUT: Event, OUTPUT: Event> {
        _input: PhantomData<INPUT>,
        _output: PhantomData<OUTPUT>,
    }

    impl<INPUT, OUTPUT> WebPostMessage<INPUT, OUTPUT>
    where
        INPUT: Send + Sync + Event,
        OUTPUT: Send + Sync + Event,
    {
        pub fn new() -> Self {
            Self {
                _input: PhantomData::default(),
                _output: PhantomData::default(),
            }
        }
    }

    impl<INPUT, OUTPUT> Plugin for WebPostMessage<INPUT, OUTPUT>
    where
        INPUT: Send + Sync + Event + DeserializeOwned + std::fmt::Debug,
        OUTPUT: Send + Sync + Event + Serialize + std::fmt::Debug,
    {
        fn build(&self, app: &mut App) {
            let (tx, rx) = unbounded::<INPUT>();
            // self.sender.set(tx);

            // let handler = Box::new(js_event) as Box<dyn Fn(_)>;
            // let closure = Closure::wrap(handler);
            // let js_func = closure.as_ref().unchecked_ref();
            // web_sys::window().unwrap().set_onmessage(Some(js_func));
            // closure.forget();

            gloo::events::EventListener::new(
                &web_sys::window().unwrap().into(),
                "message",
                move |event| {
                    let event = event.dyn_ref::<web_sys::MessageEvent>().unwrap_throw();

                    // Alternative 1, alwasy input strings but force them to be JSON ( or other format that we define)
                    // ``` js
                    // window.postMessage('"Toggle"')          -> Command::Toggle
                    // window.postMessage('{"Print":"hello"}') -> Command::Print("hello".into())
                    // ```
                    match event.data().clone().as_string() {
                        Some(datastr) => match serde_json::from_str::<INPUT>(&datastr) {
                            Ok(val) => {
                                gloo::console::log!(format!("got Alt1: {val:?}"));
                                tx.send(val);
                            }
                            Err(e) => {
                                gloo::console::log!(format!(
                                    "Alt1 Failed parsing incoming string as JSON: {:?}",
                                    e
                                ));
                            }
                        },
                        None => {
                            gloo::console::log!("Alt1 fails: incoming data was not a string");
                        }
                    }

                    // Alternative 2, always assume that the incoming stuff is pure JavaScript objects, where String is one type of object. When INPUT = Command
                    //  ``` js
                    //  window.postMessage("Toggle")          -> Command::Toggle
                    //  window.postMessage({"Print":"hello"}) -> Command::Print("hello".into())
                    //  ```
                    match serde_wasm_bindgen::from_value::<INPUT>(event.data()) {
                        Ok(cmd) => {
                            gloo::console::log!(format!("got Alt2: {cmd:?}"));
                            tx.send(cmd);
                        }
                        Err(e) => gloo::console::log!(format!("Alt2 fails")),
                    }
                },
            )
            .forget();

            app.insert_resource(ReceiverResource { rx })
                .add_event::<INPUT>()
                .add_event::<OUTPUT>()
                .add_system(postmessage_to_parent::<OUTPUT>)
                .add_system(postmessage_input::<INPUT>);
        }
    }
}

use web_communication::*;
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(WebPostMessage::<Command, PostEvent>::new())
        .add_startup_system(setup)
        .add_system(sprite_movement)
        .add_system(toggle_from_js)
        .run();
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Toggle,
    Print(String),
}

#[derive(Resource, Debug, Serialize, Deserialize)]
pub enum PostEvent {
    Test(String),
    AFloat(f32),
}

#[derive(Component)]
enum Direction {
    Up,
    Down,
}
fn toggle_from_js(mut rx: EventReader<Command>, mut sprite_position: Query<&mut Direction>) {
    for rx in rx.iter() {
        for mut logo in &mut sprite_position {
            match *logo {
                Direction::Up => *logo = Direction::Down,
                Direction::Down => *logo = Direction::Up,
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // let v = serde_wasm_bindgen::to_value(&Command::Print("hello".into())).unwrap();
    // gloo::console::log!(format!("{v:?}"));
    // let v = serde_wasm_bindgen::from_value::<Command>(v);

    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(50.).into()).into(),
            material: materials.add(ColorMaterial::from(Color::PURPLE)),
            transform: Transform::from_translation(Vec3::new(-150., 0., 0.)),
            ..default()
        })
        .insert(Direction::Up);
}

/// The sprite is animated by changing its translation depending on the time that has passed since
/// the last frame.
fn sprite_movement(
    time: Res<Time>,
    mut evt: EventWriter<PostEvent>,
    mut sprite_position: Query<(&mut Direction, &mut Transform)>,
) {
    for (mut logo, mut transform) in &mut sprite_position {
        match *logo {
            Direction::Up => transform.translation.y += 150. * time.delta_seconds(),
            Direction::Down => transform.translation.y -= 150. * time.delta_seconds(),
        }

        if transform.translation.y > 200. {
            evt.send(PostEvent::Test(format!("goin Down!")));
            evt.send(PostEvent::AFloat(transform.translation.y));
            *logo = Direction::Down;
        } else if transform.translation.y < -200. {
            evt.send(PostEvent::Test(format!("goin Up!")));
            *logo = Direction::Up;
        }
    }
}
