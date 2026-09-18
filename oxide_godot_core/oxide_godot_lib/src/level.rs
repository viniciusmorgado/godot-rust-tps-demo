mod model;

use godot::classes::input::MouseMode;
use godot::classes::{
    INode3D, Input, InputEvent, LightmapGi, LightmapGiData, Marker3D, Node, Node3D, PackedScene,
    RenderingServer, WorldEnvironment,
};
use godot::global::randi;
use godot::prelude::*;

use crate::player::Player;
use crate::red_robot::EnemyRobot;
use crate::settings::Settings;
use model::GiPlan;

#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct Level {
    base: Base<Node3D>,

    lightmap_gi: Option<Gd<LightmapGi>>,

    #[init(node = "WorldEnvironment")]
    world_environment: OnReady<Gd<WorldEnvironment>>,
    #[init(node = "RobotSpawnpoints")]
    robot_spawn_points: OnReady<Gd<Node3D>>,
    #[init(node = "PlayerSpawnpoints")]
    player_spawn_points: OnReady<Gd<Node3D>>,
    #[init(node = "SpawnedNodes")]
    spawned_nodes: OnReady<Gd<Node3D>>,
    #[init(node = "VoxelGI")]
    voxel_gi: OnReady<Gd<Node3D>>,
    #[init(node = "ReflectionProbes")]
    reflection_probes: OnReady<Gd<Node3D>>,

    #[init(val = load("res://enemies/red_robot/red_robot.tscn"))]
    robot_scene: Gd<PackedScene>,
    #[init(val = load("res://player/player.tscn"))]
    player_scene: Gd<PackedScene>,

    #[init(val = OnReady::new(|| godot::tools::get_autoload_by_name::<Settings>("Settings")))]
    settings: OnReady<Gd<Settings>>,
}

#[godot_api]
impl INode3D for Level {
    fn ready(&mut self) {
        let window = self.base().get_window().unwrap();
        let environment = self.world_environment.get_environment().unwrap();
        let scene_root: Gd<Node> = self.to_gd().upcast();
        self.settings.bind_mut().apply_graphics_settings(window, environment, scene_root);

        let graphics = self.settings.bind().graphics();
        let plan = model::gi_plan(graphics.gi_type, graphics.gi_quality, self.lightmap_gi.is_some());
        self.apply_gi_plan(plan);

        let multiplayer = self.base().get_multiplayer().unwrap();
        if multiplayer.is_server() {
            // Server will spawn the red robots
            for child in self.robot_spawn_points.get_children().iter_shared() {
                self.spawn_robot(child.cast::<Node3D>());
            }

            // Then spawn already connected players at random location
            let mut spawn_points = self.player_spawn_points.get_children();
            spawn_points.shuffle();
            let first = spawn_points.pop_front().map(|n| n.cast::<Marker3D>());
            self.add_player(1, first);
            for id in multiplayer.get_peers().as_slice() {
                let next = spawn_points.pop_front().map(|n| n.cast::<Marker3D>());
                self.add_player(*id, next);
            }

            // Then spawn/despawn players as they connect/disconnect
            multiplayer
                .signals()
                .peer_connected()
                .connect_other(&*self, |this: &mut Level, id: i64| this.add_player(id as i32, None));
            multiplayer
                .signals()
                .peer_disconnected()
                .connect_other(&*self, |this: &mut Level, id: i64| this.del_player(id as i32));
        }
    }

    fn input(&mut self, input_event: Gd<InputEvent>) {
        if input_event.is_action_pressed("quit") {
            Input::singleton().set_mouse_mode(MouseMode::VISIBLE);
            self.signals().quit().emit();
        }
    }
}

#[godot_api]
impl Level {
    #[signal]
    pub fn quit();
}

impl Level {
    /// v1: `setup_sdfgi`/`setup_voxelgi`/`setup_lightmapgi` (`level.rs:94-162`), unified via
    /// `model::gi_plan`'s pure decision, applied in the same order (research.md R3).
    fn apply_gi_plan(&mut self, plan: GiPlan) {
        self.world_environment.get_environment().unwrap().set_sdfgi_enabled(plan.sdfgi_enabled);
        self.voxel_gi.set_visible(plan.voxel_visible);
        self.reflection_probes.set_visible(plan.probes_visible);
        if plan.free_lightmap
            && let Some(mut lightmap_gi) = self.lightmap_gi.take()
        {
            lightmap_gi.queue_free();
        }
        if plan.create_lightmap {
            let mut new_gi = LightmapGi::new_alloc();
            new_gi.set_light_data(&load::<LightmapGiData>("res://level/level.lmbake"));
            new_gi.set_name("LightmapGI");
            self.lightmap_gi = Some(new_gi.clone());
            self.base_mut().add_child(&new_gi);
        }
        if let Some(visible) = plan.lightmap_visible {
            self.lightmap_gi.as_mut().unwrap().set_visible(visible);
        }
        if let Some(rays) = plan.sdfgi_rays {
            RenderingServer::singleton().environment_set_sdfgi_ray_count(rays);
        }
        if let Some(quality) = plan.voxel_quality {
            RenderingServer::singleton().voxel_gi_set_quality(quality);
        }
    }

    fn spawn_robot(&mut self, spawn_point: Gd<Node3D>) {
        let mut robot: Gd<EnemyRobot> = self.robot_scene.instantiate_as::<EnemyRobot>();
        robot.set_transform(spawn_point.get_transform());
        robot
            .signals()
            .exploded()
            .connect_other(&*self, move |this: &mut Level| this._respawn_robot(spawn_point.clone()));
        self.spawned_nodes
            .add_child_ex(&robot)
            .force_readable_name(true)
            .done();
    }

    fn _respawn_robot(&mut self, spawn_point: Gd<Node3D>) {
        let this = self.to_gd();
        godot::task::spawn(async move {
            this.get_tree()
                .create_timer(15.0)
                .signals()
                .timeout()
                .to_future()
                .await;
            if !this.is_instance_valid() {
                return;
            }
            this.clone().bind_mut().spawn_robot(spawn_point);
        });
    }

    fn del_player(&mut self, id: i32) {
        let name = id.to_string();
        if !self.spawned_nodes.has_node(&name) {
            return;
        }
        self.spawned_nodes.get_node_as::<Node>(&name).queue_free();
    }

    fn add_player(&mut self, id: i32, spawn_point: Option<Gd<Marker3D>>) {
        let spawn_point = spawn_point.unwrap_or_else(|| {
            let count = self.player_spawn_points.get_child_count();
            self.player_spawn_points
                .get_child(model::pick_spawn(randi() as i64, count as i64) as i32)
                .unwrap()
                .cast::<Marker3D>()
        });
        let mut player: Gd<Player> = self.player_scene.instantiate_as::<Player>();
        player.set_name(&id.to_string());
        player.bind_mut().set_player_id(id);
        player.set_transform(spawn_point.get_transform());
        self.spawned_nodes
            .add_child_ex(&player)
            .force_readable_name(true)
            .done();
    }
}
