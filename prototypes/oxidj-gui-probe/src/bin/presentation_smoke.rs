use wayland_client::{
    Connection, Dispatch, QueueHandle,
    globals::{GlobalListContents, registry_queue_init},
    protocol::wl_registry,
};
use wayland_protocols::wp::presentation_time::client::wp_presentation;

struct State;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_presentation::WpPresentation, ()> for State {
    fn event(
        _: &mut Self,
        _: &wp_presentation::WpPresentation,
        _: wp_presentation::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

fn main() {
    let connection = Connection::connect_to_env().expect("connect to Wayland compositor");
    let (globals, queue) = registry_queue_init::<State>(&connection).expect("read globals");
    let advertised = globals
        .contents()
        .with_list(|items| items.iter().any(|item| item.interface == "wp_presentation"));
    println!("advertised_wp_presentation={advertised}");
    match globals.bind::<wp_presentation::WpPresentation, _, _>(&queue.handle(), 1..=1, ()) {
        Ok(_proxy) => println!("bound_wp_presentation=true"),
        Err(error) => println!("bound_wp_presentation=false error={error}"),
    }
}
