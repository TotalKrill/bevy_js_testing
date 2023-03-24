//! Renders a 2D scene containing a single, moving sprite.
use std::collections::HashMap;

use async_std::channel::{unbounded, Receiver, Sender};
use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use wasm_bindgen::prelude::*;

mod web_communication {
    use std::marker::PhantomData;

    use async_std::channel::{unbounded, Receiver, Sender};
    use bevy::{ecs::event::Event, prelude::*};
    use once_cell::sync::OnceCell;
    use serde::{de::DeserializeOwned, Serialize};
    use wasm_bindgen::prelude::*;

    /// Wrapper around Receiver, just to derive [`Resource`].
    #[derive(Resource)]
    struct ReceiverResource<T> {
        rx: Receiver<T>,
    }

    /// system that sends postMessage to the parent window
    pub fn postmessage_to_parent<OUTPUT: Send + Sync + Event + Serialize>(
        mut postevt: EventReader<OUTPUT>,
    ) {
        for evt in postevt.iter() {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(parent)) = window.parent() {
                    parent.post_message(&serde_wasm_bindgen::to_value(evt).unwrap(), "*");
                } else {
                    window.post_message(&serde_wasm_bindgen::to_value(evt).unwrap(), "*");
                }
            }
        }
    }
    pub fn postmessage_input<INPUT: Send + Sync + Event>(
        rx: Res<ReceiverResource<INPUT>>,
        evt: EventWriter<INPUT>,
    ) {
        if let Ok(rx) = rx.rx.try_recv() {
            evt.send(rx);
        }
    }

    use crate::SENDER;

    #[wasm_bindgen]
    extern "C" {
        pub async fn js_event(val: JsValue) {
            if let Ok(cmd) = serde_wasm_bindgen::from_value(val) {
                if let Some(sender) = SENDER.get() {
                    sender.send(cmd).await;
                }
            }
        }
    }

    pub struct WebPostMessage<INPUT: Event, OUTPUT: Event> {
        sender: &'static OnceCell<Sender<INPUT>>,
        _input: PhantomData<INPUT>,
        _output: PhantomData<OUTPUT>,
    }

    impl<INPUT, OUTPUT> WebPostMessage<INPUT, OUTPUT>
    where
        INPUT: Send + Sync + Event,
        OUTPUT: Send + Sync + Event,
    {
        pub fn new(sender: &'static OnceCell<Sender<INPUT>>) -> Self {
            Self {
                sender,
                _input: PhantomData::default(),
                _output: PhantomData::default(),
            }
        }
    }

    impl<INPUT, OUTPUT> Plugin for WebPostMessage<INPUT, OUTPUT>
    where
        INPUT: Send + Sync + Event + DeserializeOwned,
        OUTPUT: Send + Sync + Event + Serialize,
    {
        fn build(&self, app: &mut App) {
            let (tx, rx) = unbounded();
            self.sender.set(tx);

            let cb = Closure::new(|val| js_event(val));
            let cb = cb.as_ref().unchecked_ref();

            web_sys::window().unwrap().set_onmessage(Some(cb));

            app.insert_resource(ReceiverResource { rx })
                .add_event::<INPUT>()
                .add_event::<OUTPUT>()
                .add_system(postmessage_to_parent::<OUTPUT>)
                .add_system(postmessage_input::<INPUT>);
        }
    }
}
static SENDER: OnceCell<Sender<Command>> = OnceCell::new();

use web_communication::*;
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(WebPostMessage::<Command, PostEvent>::new(&SENDER))
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
fn toggle_from_js(rx: EventReader<Command>, mut sprite_position: Query<&mut Direction>) {
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
            *logo = Direction::Down;
        } else if transform.translation.y < -200. {
            evt.send(PostEvent::Test(format!("goin Up!")));
            *logo = Direction::Up;
        }
    }
}
