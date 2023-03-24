//! Renders a 2D scene containing a single, moving sprite.
use std::collections::HashMap;

use async_std::channel::{unbounded, Receiver, Sender};
use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use wasm_bindgen::prelude::*;
/// Wrapper around Receiver, just to derive [`Resource`].
#[derive(Resource)]
struct ReceiverResource<T> {
    rx: Receiver<T>,
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

/// system that sends postMessage to the parent window
pub fn postmessage_to_parent(mut postevt: EventReader<PostEvent>) {
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

#[wasm_bindgen]
pub async fn js_event(val: JsValue) {
    let cmd: Command = serde_wasm_bindgen::from_value(val).unwrap();

    match cmd {
        Command::Toggle => {
            if let Some(sender) = SENDER.get() {
                sender.send(cmd).await;
            }
        }
        Command::Print(s) => {
            web_sys::console::log_1(&s.into());
        }
    }
}

static SENDER: OnceCell<Sender<Command>> = OnceCell::new();

fn toggle_js(rx: Res<ReceiverResource<Command>>, mut sprite_position: Query<&mut Direction>) {
    if let Ok(Command::Toggle) = rx.rx.try_recv() {
        for mut logo in &mut sprite_position {
            match *logo {
                Direction::Up => *logo = Direction::Down,
                Direction::Down => *logo = Direction::Up,
            }
        }
    }
}

fn main() {
    let (tx, rx) = unbounded();
    SENDER.set(tx);

    App::new()
        .insert_resource(ReceiverResource { rx })
        .add_plugins(DefaultPlugins)
        .add_event::<PostEvent>()
        .add_startup_system(setup)
        .add_system(sprite_movement)
        .add_system(toggle_js)
        .add_system(postmessage_to_parent)
        .run();
}

#[derive(Component)]
enum Direction {
    Up,
    Down,
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
