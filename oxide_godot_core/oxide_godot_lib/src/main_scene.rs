use crate::level::Level;
use crate::menu::Menu;
use crate::settings::Settings;
use godot::classes::{
    DisplayServer, Engine, INode, MultiplayerPeer, Node, OfflineMultiplayerPeer, PackedScene,
    ResourceLoader, SceneMultiplayer,
};
use godot::global::randomize;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct Main {
    base: Base<Node>,

    #[init(val = OnReady::new(|| godot::tools::get_autoload_by_name::<Settings>("Settings")))]
    settings: OnReady<Gd<Settings>>,
}

#[godot_api]
impl INode for Main {
    fn ready(&mut self) {
        self.base()
            .get_multiplayer()
            .unwrap()
            .cast::<SceneMultiplayer>()
            .set_server_relay_enabled(false);
        if DisplayServer::singleton().get_name() == "headless" {
            Engine::singleton().set_max_fps(60);
        }
        randomize();
        let display_mode = self.settings.bind().graphics().display_mode;
        self.base().get_window().unwrap().set_mode(display_mode);
        self.go_to_main_menu();
    }
}

impl Main {
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

    fn replace_main_scene(&mut self, resource: Gd<PackedScene>) {
        let mut this = self.to_gd();
        Callable::from_fn("change_scene_to_packed", move |_args| {
            this.bind_mut().change_scene_to_packed(resource.clone());
            Variant::nil()
        })
        .call_deferred(&[]);
    }

    fn change_scene_to_packed(&mut self, resource: Gd<PackedScene>) {
        let node = resource.instantiate().unwrap();
        for mut child in self.base().get_children().iter_shared() {
            self.base_mut().remove_child(&child);
            child.queue_free();
        }
        let this = self.to_gd();
        if let Ok(level) = node.clone().try_cast::<Level>() {
            level
                .signals()
                .quit()
                .connect_other(&this, |main: &mut Main| main.go_to_main_menu());
        } else if let Ok(menu) = node.clone().try_cast::<Menu>() {
            menu.signals().replace_main_scene().connect_other(
                &this,
                |main: &mut Main, scene: Gd<PackedScene>| main.replace_main_scene(scene),
            );
        }
        self.base_mut().add_child(&node);
    }
}
