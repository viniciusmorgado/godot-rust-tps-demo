use godot::classes::window::Mode as WindowMode;
use godot::classes::{
    ConfigFile, DisplayServer, Engine, INode, MultiplayerPeer, Node, OfflineMultiplayerPeer,
    PackedScene, ResourceLoader, SceneMultiplayer,
};
use godot::global::randomize;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct Main {
    base: Base<Node>,
}

#[godot_api]
impl INode for Main {
    fn ready(&mut self) {
        self.base()
            .get_multiplayer()
            .unwrap()
            .cast::<SceneMultiplayer>()
            .set_server_relay_enabled(false);
        if DisplayServer::singleton().get_name() == GString::from("headless") {
            Engine::singleton().set_max_fps(60);
        }
        randomize();
        let display_mode = self
            .base()
            .get_node_as::<Node>("/root/Settings")
            .get("config_file")
            .to::<Gd<ConfigFile>>()
            .get_value("video", "display_mode")
            .to::<i64>();
        self.base()
            .get_window()
            .unwrap()
            .set_mode(WindowMode::from_ord(display_mode as i32));
        self.go_to_main_menu();
    }
}

#[godot_api]
impl Main {
    #[func]
    fn go_to_main_menu(&mut self) {
        let menu = ResourceLoader::singleton()
            .load("res://menu/menu.tscn")
            .unwrap()
            .cast::<PackedScene>();
        let mut multiplayer = self.base().get_multiplayer().unwrap();
        multiplayer.get_multiplayer_peer().unwrap().close();
        multiplayer.set_multiplayer_peer(&OfflineMultiplayerPeer::new_gd().upcast::<MultiplayerPeer>());
        self.change_scene_to_packed(menu);
    }

    #[func]
    fn replace_main_scene(&mut self, resource: Gd<PackedScene>) {
        self.base_mut()
            .call_deferred("change_scene_to_packed", &[resource.to_variant()]);
    }

    #[func]
    fn change_scene_to_packed(&mut self, resource: Gd<PackedScene>) {
        let mut node = resource.instantiate().unwrap();
        for mut child in self.base().get_children().iter_shared() {
            self.base_mut().remove_child(&child);
            child.queue_free();
        }
        self.base_mut().add_child(&node);
        if node.has_signal("quit") {
            node.connect("quit", &Callable::from_object_method(&self.to_gd(), "go_to_main_menu"));
        }
        if node.has_signal("replace_main_scene") {
            node.connect(
                "replace_main_scene",
                &Callable::from_object_method(&self.to_gd(), "replace_main_scene"),
            );
        }
    }
}
