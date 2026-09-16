use godot::classes::input::MouseMode;
use godot::classes::rendering_server::{EnvironmentSdfgiRayCount, VoxelGiQuality};
use godot::classes::{
    INode3D, Input, InputEvent, LightmapGi, LightmapGiData, Marker3D, Node, Node3D, PackedScene,
    RenderingServer, WorldEnvironment,
};
use godot::global::{randi, randomize};
use godot::prelude::*;

use crate::player::Player;
use crate::red_robot::EnemyRobot;
use crate::settings::{GiQuality, GiType, Settings};

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

        let gi_type = self.settings.bind().graphics().gi_type;
        match gi_type {
            GiType::Sdfgi => self.setup_sdfgi(),
            GiType::VoxelGi => self.setup_voxelgi(),
            GiType::LightmapGi => self.setup_lightmapgi(),
        }

        let multiplayer = self.base().get_multiplayer().unwrap();
        if multiplayer.is_server() {
            // Server will spawn the red robots
            for child in self.robot_spawn_points.get_children().iter_shared() {
                self.spawn_robot(child.cast::<Node3D>());
            }

            // Then spawn already connected players at random location
            randomize();
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
    fn quit();
}

impl Level {
    fn setup_sdfgi(&mut self) {
        self.world_environment.get_environment().unwrap().set_sdfgi_enabled(true);
        self.base().get_node_as::<Node3D>("VoxelGI").hide();
        self.base().get_node_as::<Node3D>("ReflectionProbes").hide();
        // LightmapGI nodes override SDFGI (even when hidden)
        // so we need to free the LightmapGI node if it exists
        if let Some(lightmap_gi) = &mut self.lightmap_gi {
            lightmap_gi.queue_free();
        }

        let gi_quality = self.settings.bind().graphics().gi_quality;
        match gi_quality {
            GiQuality::High => {
                RenderingServer::singleton()
                    .environment_set_sdfgi_ray_count(EnvironmentSdfgiRayCount::COUNT_96);
            }
            GiQuality::Low => {
                RenderingServer::singleton()
                    .environment_set_sdfgi_ray_count(EnvironmentSdfgiRayCount::COUNT_32);
            }
            GiQuality::Disabled => {
                self.world_environment.get_environment().unwrap().set_sdfgi_enabled(false);
            }
        }
    }

    fn setup_voxelgi(&mut self) {
        self.world_environment.get_environment().unwrap().set_sdfgi_enabled(false);
        self.base().get_node_as::<Node3D>("VoxelGI").show();
        self.base().get_node_as::<Node3D>("ReflectionProbes").hide();
        // LightmapGI nodes override VoxelGI (even when hidden)
        // so we need to free the LightmapGI node if it exists
        if let Some(lightmap_gi) = &mut self.lightmap_gi {
            lightmap_gi.queue_free();
        }

        let gi_quality = self.settings.bind().graphics().gi_quality;
        match gi_quality {
            GiQuality::High => {
                RenderingServer::singleton().voxel_gi_set_quality(VoxelGiQuality::HIGH);
            }
            GiQuality::Low => {
                RenderingServer::singleton().voxel_gi_set_quality(VoxelGiQuality::LOW);
            }
            GiQuality::Disabled => {
                self.base().get_node_as::<Node3D>("VoxelGI").hide();
            }
        }
    }

    fn setup_lightmapgi(&mut self) {
        self.world_environment.get_environment().unwrap().set_sdfgi_enabled(false);
        self.base().get_node_as::<Node3D>("VoxelGI").hide();
        self.base().get_node_as::<Node3D>("ReflectionProbes").show();
        // If no LightmapGI node, create one
        if self.lightmap_gi.is_none() {
            let mut new_gi = LightmapGi::new_alloc();
            new_gi.set_light_data(&load::<LightmapGiData>("res://level/level.lmbake"));
            new_gi.set_name("LightmapGI");
            self.lightmap_gi = Some(new_gi.clone());
            self.base_mut().add_child(&new_gi);
        }

        let gi_quality = self.settings.bind().graphics().gi_quality;
        if gi_quality == GiQuality::Disabled {
            self.lightmap_gi.as_mut().unwrap().hide();
            self.base().get_node_as::<Node3D>("ReflectionProbes").hide();
        }
    }

    fn spawn_robot(&mut self, spawn_point: Gd<Node3D>) {
        let mut robot: Gd<EnemyRobot> =
            load::<PackedScene>("res://enemies/red_robot/red_robot.tscn").instantiate_as::<EnemyRobot>();
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
        self.base()
            .get_tree()
            .create_timer(15.0)
            .signals()
            .timeout()
            .connect_other(&*self, move |this: &mut Level| this.spawn_robot(spawn_point.clone()));
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
                .get_child((randi() % count as i64) as i32)
                .unwrap()
                .cast::<Marker3D>()
        });
        let mut player: Gd<Player> =
            load::<PackedScene>("res://player/player.tscn").instantiate_as::<Player>();
        player.set_name(&id.to_string());
        player.bind_mut().set_player_id(id);
        player.set_transform(spawn_point.get_transform());
        self.spawned_nodes.add_child(&player);
    }
}
