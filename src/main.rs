use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    post_process::bloom::Bloom,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    window::PrimaryWindow,
};
use neon_range::{MUZZLE, TARGET_Z, difficulty, segment_hits_sphere};

const CYAN: Color = Color::srgb(0.25, 0.94, 0.91);
const WHITE: Color = Color::srgb(0.9, 0.96, 1.0);
const MUTED: Color = Color::srgb(0.43, 0.57, 0.65);

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Menu,
    Playing,
    Paused,
    Over,
}
#[derive(Resource)]
struct Run {
    phase: Phase,
    score: u32,
    best: u32,
    level: u32,
    kills: u32,
    lives: u32,
    shots: u32,
    hits: u32,
    combo: u32,
    spawn: f32,
    cooldown: f32,
    elapsed: f32,
    banner: f32,
    flash: f32,
    serial: u32,
    aim: Vec2,
}
impl Default for Run {
    fn default() -> Self {
        Self {
            phase: Phase::Menu,
            score: 0,
            best: 0,
            level: 1,
            kills: 0,
            lives: 5,
            shots: 0,
            hits: 0,
            combo: 0,
            spawn: 0.4,
            cooldown: 0.0,
            elapsed: 0.0,
            banner: 3.0,
            flash: 0.0,
            serial: 0,
            aim: Vec2::new(0.0, 4.0),
        }
    }
}
#[derive(Resource)]
struct Art {
    cube: Handle<Mesh>,
    sphere: Handle<Mesh>,
    ring: Handle<Mesh>,
    metal: Handle<StandardMaterial>,
    cyan: Handle<StandardMaterial>,
    orange: Handle<StandardMaterial>,
    pink: Handle<StandardMaterial>,
}
#[derive(Component)]
struct Target {
    velocity: f32,
    base_y: f32,
    age: f32,
    radius: f32,
}
#[derive(Component)]
struct Bullet {
    velocity: Vec3,
    ttl: f32,
}
#[derive(Component)]
struct Spark {
    velocity: Vec3,
    ttl: f32,
}
#[derive(Component)]
struct Transient;
#[derive(Component)]
struct Weapon;
#[derive(Component)]
struct Reticle;
#[derive(Component)]
struct Hud;
#[derive(Component)]
struct Status;
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct OverlayTitle;
#[derive(Component)]
struct OverlayBody;
#[derive(Component)]
struct Progress;
#[derive(Component)]
struct Damage;
#[derive(Resource, Default)]
struct Smoke {
    enabled: bool,
    frames: u32,
}

fn main() {
    let smoke = std::env::args().any(|a| a == "--smoke");
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.012, 0.022, 0.043)))
        .insert_resource(Run::default())
        .insert_resource(Smoke {
            enabled: smoke,
            ..default()
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "NEON RANGE / Precision Division".into(),
                resolution: (1440, 900).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (scene, interface).chain())
        .add_systems(
            Update,
            (controls, simulate, effects, update_ui, smoke_test).chain(),
        )
        .run();
}

fn material(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
    glow: f32,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        emissive: color.to_linear() * glow,
        metallic: 0.35,
        perceptual_roughness: 0.35,
        ..default()
    })
}
fn block(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    mat: &Handle<StandardMaterial>,
    pos: Vec3,
    scale: Vec3,
) {
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(pos).with_scale(scale),
    ));
}
fn scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 6.0, 20.0).looking_at(Vec3::new(0.0, 3.5, -10.0), Vec3::Y),
        Projection::Perspective(PerspectiveProjection {
            fov: 54.0_f32.to_radians(),
            ..default()
        }),
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.4, 0.6, 0.85),
        brightness: 180.0,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            illuminance: 3500.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(5.0, 12.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    for (x, color) in [(-10.0, CYAN), (10.0, Color::srgb(1.0, 0.3, 0.15))] {
        commands.spawn((
            PointLight {
                color,
                intensity: 140_000.0,
                range: 32.0,
                ..default()
            },
            Transform::from_xyz(x, 5.0, -6.0),
        ));
    }
    let cube = meshes.add(Cuboid::default());
    let sphere = meshes.add(Sphere::new(1.0).mesh().ico(3).unwrap());
    let ring = meshes.add(Torus::new(0.77, 1.0));
    let metal = material(&mut materials, Color::srgb(0.07, 0.11, 0.16), 0.0);
    let floor = material(&mut materials, Color::srgb(0.028, 0.045, 0.065), 0.0);
    let cyan = material(&mut materials, CYAN, 4.0);
    let grid = material(&mut materials, Color::srgb(0.025, 0.2, 0.23), 0.6);
    let orange = material(&mut materials, Color::srgb(1.0, 0.36, 0.08), 3.5);
    let pink = material(&mut materials, Color::srgb(1.0, 0.08, 0.36), 4.0);
    block(
        &mut commands,
        &cube,
        &floor,
        Vec3::new(0.0, -0.3, -5.0),
        Vec3::new(34.0, 0.5, 48.0),
    );
    block(
        &mut commands,
        &cube,
        &metal,
        Vec3::new(0.0, 5.0, -18.5),
        Vec3::new(32.0, 11.0, 0.5),
    );
    for x in -8..=8 {
        block(
            &mut commands,
            &cube,
            &grid,
            Vec3::new(x as f32 * 2.0, -0.035, -5.0),
            Vec3::new(0.024, 0.018, 45.0),
        );
        block(
            &mut commands,
            &cube,
            &grid,
            Vec3::new(x as f32 * 2.0, 5.0, -18.2),
            Vec3::new(0.025, 10.0, 0.02),
        );
    }
    for z in -9..=9 {
        block(
            &mut commands,
            &cube,
            &grid,
            Vec3::new(0.0, -0.03, z as f32 * 2.0),
            Vec3::new(32.0, 0.018, 0.024),
        );
    }
    for y in 0..=5 {
        block(
            &mut commands,
            &cube,
            &grid,
            Vec3::new(0.0, y as f32 * 2.0, -18.2),
            Vec3::new(32.0, 0.025, 0.02),
        );
    }
    // Portal frames and luminous side rails establish the depth of the gallery.
    for z in [-16.0, -8.0, 0.0, 8.0] {
        for x in [-14.0, 14.0] {
            block(
                &mut commands,
                &cube,
                &metal,
                Vec3::new(x, 5.0, z),
                Vec3::new(0.5, 10.0, 0.55),
            );
            block(
                &mut commands,
                &cube,
                &cyan,
                Vec3::new(x * 0.986, 5.0, z + 0.3),
                Vec3::new(0.055, 9.5, 0.035),
            );
        }
        block(
            &mut commands,
            &cube,
            &metal,
            Vec3::new(0.0, 10.0, z),
            Vec3::new(28.0, 0.4, 0.6),
        );
    }
    for x in [-12.5, 12.5] {
        block(
            &mut commands,
            &cube,
            &cyan,
            Vec3::new(x, 0.05, -4.0),
            Vec3::new(0.06, 0.04, 28.0),
        );
        block(
            &mut commands,
            &cube,
            &orange,
            Vec3::new(x, 4.5, TARGET_Z),
            Vec3::new(0.08, 7.0, 0.08),
        );
    }
    for y in [1.5, 4.0, 6.5] {
        block(
            &mut commands,
            &cube,
            &grid,
            Vec3::new(0.0, y, -13.5),
            Vec3::new(24.0, 0.04, 0.06),
        );
    }
    // Stationary turret. The barrel rotates toward the keyboard-controlled sight.
    block(
        &mut commands,
        &cube,
        &metal,
        Vec3::new(0.0, 0.3, 8.0),
        Vec3::new(2.5, 0.6, 2.5),
    );
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.85, 0.65))),
        MeshMaterial3d(metal.clone()),
        Transform::from_xyz(0.0, 0.9, 8.0),
    ));
    commands
        .spawn((
            Weapon,
            Transform::from_translation(MUZZLE),
            Visibility::default(),
        ))
        .with_children(|p| {
            p.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(metal.clone()),
                Transform::from_xyz(0.0, 0.0, -0.4).with_scale(Vec3::new(0.65, 0.65, 1.9)),
            ));
            for x in [-0.23, 0.23] {
                p.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(cyan.clone()),
                    Transform::from_xyz(x, 0.12, -1.25).with_scale(Vec3::new(0.07, 0.07, 1.4)),
                ));
            }
            p.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(metal.clone()),
                Transform::from_xyz(0.0, 0.0, -1.6).with_scale(Vec3::new(0.5, 0.45, 0.25)),
            ));
        });
    commands.insert_resource(Art {
        cube,
        sphere,
        ring,
        metal,
        cyan,
        orange,
        pink,
    });
}

fn label(text: impl Into<String>, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (
        Text::new(text),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(color),
    )
}
fn interface(mut commands: Commands) {
    commands
        .spawn((Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::all(px(32)),
            ..default()
        },))
        .with_children(|root| {
            root.spawn((Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Start,
                ..default()
            },))
                .with_children(|top| {
                    top.spawn((Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(6),
                        ..default()
                    },))
                        .with_children(|brand| {
                            brand.spawn(label("N E O N  /  R A N G E", 26.0, WHITE));
                            brand.spawn(label(
                                "PRECISION DIVISION     /     TRAINING BAY 07",
                                11.0,
                                CYAN,
                            ));
                        });
                    top.spawn((
                        Hud,
                        label("", 19.0, WHITE),
                        TextLayout::new_with_justify(Justify::Right),
                    ));
                });
            root.spawn((Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            },))
                .with_children(|bottom| {
                    bottom.spawn((Status, label("", 15.0, CYAN)));
                    bottom
                        .spawn((
                            Node {
                                width: percent(100),
                                height: px(3),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.1, 0.3, 0.35, 0.6)),
                        ))
                        .with_children(|bar| {
                            bar.spawn((
                                Progress,
                                Node {
                                    width: percent(0),
                                    height: percent(100),
                                    ..default()
                                },
                                BackgroundColor(CYAN),
                            ));
                        });
                    bottom
                        .spawn((Node {
                            justify_content: JustifyContent::SpaceBetween,
                            ..default()
                        },))
                        .with_children(|row| {
                            row.spawn(label(
                                "WASD / ARROWS   Aim       SPACE   Fire       SHIFT   Precision",
                                13.0,
                                MUTED,
                            ));
                            row.spawn(label(
                                "P / ESC   Pause       R   Restart       F11   Fullscreen",
                                13.0,
                                MUTED,
                            ));
                        });
                });
        });
    commands
        .spawn((
            Reticle,
            Node {
                position_type: PositionType::Absolute,
                width: px(30),
                height: px(30),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(15)),
                ..default()
            },
            BorderColor::all(CYAN),
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(13),
                    top: px(13),
                    width: px(2),
                    height: px(2),
                    ..default()
                },
                BackgroundColor(WHITE),
            ));
            for (left, top, width, height) in [
                (14.0, -7.0, 1.0, 7.0),
                (14.0, 29.0, 1.0, 7.0),
                (-7.0, 14.0, 7.0, 1.0),
                (29.0, 14.0, 7.0, 1.0),
            ] {
                p.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(left),
                        top: px(top),
                        width: px(width),
                        height: px(height),
                        ..default()
                    },
                    BackgroundColor(CYAN),
                ));
            }
        });
    commands
        .spawn((
            Overlay,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.008, 0.016, 0.03, 0.78)),
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    width: px(590),
                    padding: UiRect::all(px(40)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(24),
                    border: UiRect::left(px(3)),
                    ..default()
                },
                BorderColor::all(CYAN),
                BackgroundColor(Color::srgba(0.025, 0.045, 0.07, 0.94)),
            ))
            .with_children(|panel| {
                panel.spawn(label("01   /   PRECISION IS EVERYTHING", 12.0, CYAN));
                panel.spawn((OverlayTitle, label("LOCK IN.\nLIGHT IT UP.", 54.0, WHITE)));
                panel.spawn((OverlayBody, label("", 18.0, MUTED)));
                panel.spawn(label("[ ENTER ]   ENGAGE", 18.0, CYAN));
            });
        });
    commands.spawn((
        Damage,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(Color::NONE),
    ));
}

fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut commands: Commands,
    transients: Query<Entity, With<Transient>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    smoke: Res<Smoke>,
) {
    if keys.just_pressed(KeyCode::F11) {
        for mut w in &mut window {
            w.mode = match w.mode {
                bevy::window::WindowMode::Windowed => {
                    bevy::window::WindowMode::BorderlessFullscreen(
                        bevy::window::MonitorSelection::Current,
                    )
                }
                _ => bevy::window::WindowMode::Windowed,
            };
        }
    }
    if keys.just_pressed(KeyCode::KeyR)
        || (keys.just_pressed(KeyCode::Enter) && matches!(run.phase, Phase::Menu | Phase::Over))
    {
        let best = run.best.max(run.score);
        *run = Run {
            phase: Phase::Playing,
            best,
            ..default()
        };
        for e in &transients {
            commands.entity(e).despawn();
        }
    } else if keys.just_pressed(KeyCode::KeyP)
        || keys.just_pressed(KeyCode::Escape)
        || (keys.just_pressed(KeyCode::Enter) && run.phase == Phase::Paused)
    {
        run.phase = match run.phase {
            Phase::Playing => Phase::Paused,
            Phase::Paused => Phase::Playing,
            phase => phase,
        };
    }
    if !smoke.enabled && window.iter().any(|w| !w.focused) && run.phase == Phase::Playing {
        run.phase = Phase::Paused;
    }
    if run.phase != Phase::Playing {
        return;
    }
    let axis = |a, b| {
        if keys.pressed(a) || keys.pressed(b) {
            1.0
        } else {
            0.0
        }
    };
    let direction = Vec2::new(
        axis(KeyCode::KeyD, KeyCode::ArrowRight) - axis(KeyCode::KeyA, KeyCode::ArrowLeft),
        axis(KeyCode::KeyW, KeyCode::ArrowUp) - axis(KeyCode::KeyS, KeyCode::ArrowDown),
    )
    .normalize_or_zero();
    let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        3.0
    } else {
        10.0
    };
    run.aim += direction * speed * time.delta_secs().min(0.05);
    run.aim = run.aim.clamp(Vec2::new(-12.0, 0.8), Vec2::new(12.0, 8.0));
}

fn spawn_target(commands: &mut Commands, art: &Art, run: &mut Run) {
    let d = difficulty(run.level);
    let i = run.serial;
    run.serial += 1;
    let direction = if i.is_multiple_of(2) { 1.0 } else { -1.0 };
    let y = 2.0 + ((i * 7 + 3) % 11) as f32 * 0.43;
    let color = if i.is_multiple_of(3) {
        &art.pink
    } else {
        &art.orange
    };
    commands
        .spawn((
            Target {
                velocity: direction * d.speed,
                base_y: y,
                age: i as f32,
                radius: d.radius,
            },
            Transient,
            Transform::from_xyz(-direction * 11.8, y, TARGET_Z),
            Visibility::default(),
        ))
        .with_children(|p| {
            p.spawn((
                Mesh3d(art.sphere.clone()),
                MeshMaterial3d(art.metal.clone()),
                Transform::from_scale(Vec3::splat(d.radius * 0.7)),
            ));
            p.spawn((
                Mesh3d(art.ring.clone()),
                MeshMaterial3d(color.clone()),
                Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
                    .with_scale(Vec3::splat(d.radius)),
            ));
            p.spawn((
                Mesh3d(art.sphere.clone()),
                MeshMaterial3d(color.clone()),
                Transform::from_xyz(0.0, 0.0, d.radius * 0.55)
                    .with_scale(Vec3::splat(d.radius * 0.24)),
            ));
            for x in [-1.0, 1.0] {
                p.spawn((
                    Mesh3d(art.cube.clone()),
                    MeshMaterial3d(art.metal.clone()),
                    Transform::from_xyz(x * d.radius * 1.15, 0.0, 0.0).with_scale(Vec3::new(
                        d.radius * 0.7,
                        0.16,
                        0.28,
                    )),
                ));
                p.spawn((
                    Mesh3d(art.cube.clone()),
                    MeshMaterial3d(color.clone()),
                    Transform::from_xyz(x * d.radius * 1.45, 0.0, 0.0)
                        .with_scale(Vec3::new(0.07, 0.27, 0.3)),
                ));
            }
        });
}
fn burst(commands: &mut Commands, art: &Art, pos: Vec3) {
    for i in 0..16 {
        let t = i as f32 * 2.39996;
        let velocity = Vec3::new(t.cos(), t.sin(), (t * 3.0).sin()) * (2.2 + (i % 4) as f32);
        commands.spawn((
            Transient,
            Spark { velocity, ttl: 0.6 },
            Mesh3d(art.cube.clone()),
            MeshMaterial3d(art.cyan.clone()),
            Transform::from_translation(pos).with_scale(Vec3::splat(0.10)),
        ));
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn simulate(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    art: Res<Art>,
    mut run: ResMut<Run>,
    mut targets: Query<(Entity, &mut Target, &mut Transform), (Without<Bullet>, Without<Weapon>)>,
    mut bullets: Query<(Entity, &mut Bullet, &mut Transform), (Without<Target>, Without<Weapon>)>,
    mut weapon: Query<&mut Transform, (With<Weapon>, Without<Target>, Without<Bullet>)>,
) {
    for mut transform in &mut weapon {
        transform.look_at(Vec3::new(run.aim.x, run.aim.y, TARGET_Z), Vec3::Y);
    }
    if run.phase != Phase::Playing {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    run.elapsed += dt;
    run.banner = (run.banner - dt).max(0.0);
    run.cooldown = (run.cooldown - dt).max(0.0);
    run.spawn -= dt;
    let d = difficulty(run.level);
    if run.spawn <= 0.0 {
        spawn_target(&mut commands, &art, &mut run);
        run.spawn = d.interval;
    }
    if keys.pressed(KeyCode::Space) && run.cooldown <= 0.0 {
        let dir = (Vec3::new(run.aim.x, run.aim.y, TARGET_Z) - MUZZLE).normalize();
        commands.spawn((
            Transient,
            Bullet {
                velocity: dir * 65.0,
                ttl: 0.8,
            },
            Mesh3d(art.cube.clone()),
            MeshMaterial3d(art.cyan.clone()),
            Transform::from_translation(MUZZLE + dir * 1.8)
                .looking_to(dir, Vec3::Y)
                .with_scale(Vec3::new(0.06, 0.06, 1.25)),
        ));
        run.cooldown = 0.18;
        run.shots += 1;
    }
    let mut removed = Vec::new();
    for (e, mut target, mut transform) in &mut targets {
        target.age += dt;
        transform.translation.x += target.velocity * dt;
        transform.translation.y = target.base_y + (target.age * 2.0).sin() * d.weave;
        transform.rotation = Quat::from_rotation_z((target.age * 1.6).sin() * 0.12);
        if transform.translation.x.abs() > 13.0 {
            commands.entity(e).despawn();
            removed.push(e);
            run.lives = run.lives.saturating_sub(1);
            run.combo = 0;
            run.flash = 0.45;
        }
    }
    for (entity, mut bullet, mut transform) in &mut bullets {
        let start = transform.translation;
        let end = start + bullet.velocity * dt;
        bullet.ttl -= dt;
        let hit = targets
            .iter()
            .find(|(e, target, t)| {
                !removed.contains(e)
                    && segment_hits_sphere(start, end, t.translation, target.radius + 0.08)
            })
            .map(|(e, _, t)| (e, t.translation));
        if let Some((e, pos)) = hit {
            commands.entity(e).despawn();
            removed.push(e);
            commands.entity(entity).despawn();
            burst(&mut commands, &art, pos);
            run.hits += 1;
            run.kills += 1;
            run.combo += 1;
            run.score += 100 * run.level * (1 + run.combo / 5).min(4);
            run.best = run.best.max(run.score);
        } else if bullet.ttl <= 0.0 {
            commands.entity(entity).despawn();
            run.combo = 0;
        } else {
            transform.translation = end;
        }
    }
    if run.lives == 0 {
        run.phase = Phase::Over;
    } else if run.kills >= d.quota {
        run.level += 1;
        run.kills = 0;
        run.banner = 3.0;
        run.spawn = 1.8;
        run.lives = (run.lives + 1).min(5);
        for (e, _, _) in &targets {
            if !removed.contains(&e) {
                commands.entity(e).despawn();
            }
        }
        for (e, _, _) in &bullets {
            commands.entity(e).try_despawn();
        }
    }
}

fn effects(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut sparks: Query<(Entity, &mut Spark, &mut Transform)>,
) {
    if run.phase == Phase::Paused {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    run.flash = (run.flash - dt).max(0.0);
    for (e, mut spark, mut transform) in &mut sparks {
        spark.ttl -= dt;
        if spark.ttl <= 0.0 {
            commands.entity(e).despawn();
        } else {
            spark.velocity.y -= 5.0 * dt;
            transform.translation += spark.velocity * dt;
            transform.scale = Vec3::splat(spark.ttl * 0.18);
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_ui(
    run: Res<Run>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut text: ParamSet<(
        Query<&mut Text, With<Hud>>,
        Query<&mut Text, With<Status>>,
        Query<&mut Text, With<OverlayTitle>>,
        Query<&mut Text, With<OverlayBody>>,
    )>,
    mut nodes: ParamSet<(
        Query<&mut Node, With<Overlay>>,
        Query<&mut Node, With<Reticle>>,
        Query<&mut Node, With<Progress>>,
    )>,
    mut damage: Query<&mut BackgroundColor, With<Damage>>,
) {
    let accuracy = if run.shots == 0 {
        100
    } else {
        run.hits * 100 / run.shots
    };
    for mut t in &mut text.p0() {
        t.0 = format!(
            "SCORE  {:06}     /     BEST  {:06}\nLEVEL  {:02}     INTEGRITY  {} / 5     ACC  {}%",
            run.score, run.best, run.level, run.lives, accuracy
        );
    }
    for mut t in &mut text.p1() {
        t.0 = if run.phase == Phase::Menu {
            "SYSTEM READY   /   AWAITING OPERATOR".into()
        } else if run.banner > 0.0 {
            format!(
                "LEVEL {:02}   /   DESTROY {} DRONES     +     CLEAR A WAVE TO REPAIR INTEGRITY",
                run.level,
                difficulty(run.level).quota
            )
        } else {
            format!(
                "TARGETS  {:02} / {:02}       STREAK  {:02}       MULTIPLIER  x{}",
                run.kills,
                difficulty(run.level).quota,
                run.combo,
                (1 + run.combo / 5).min(4)
            )
        };
    }
    for mut n in &mut nodes.p0() {
        n.display = if run.phase == Phase::Playing {
            Display::None
        } else {
            Display::Flex
        };
    }
    for mut t in &mut text.p2() {
        t.0 = match run.phase {
            Phase::Menu => "LOCK IN.\nLIGHT IT UP.",
            Phase::Paused => "HOLD\nPOSITION.",
            Phase::Over => "SESSION\nCOMPLETE.",
            _ => "",
        }
        .into();
    }
    for mut t in &mut text.p3() {
        t.0 = match run.phase {
            Phase::Menu => "Track moving drones. Lead your shots.\nKeep them from escaping the range.\n\nWASD / arrows to aim. Hold SPACE to fire.\nHold SHIFT for fine aim. Five escapes end a run.\n\nEach level: faster drones, smaller targets,\nand more unpredictable flight paths.".into(),
            Phase::Paused => "Simulation paused. Take a breath.\n\nPress ENTER, P or ESC to resume.\nPress R to begin a fresh session.".into(),
            Phase::Over => format!("Score  {:06}     /     Best  {:06}\nLevel  {}     /     Accuracy  {}%\n\nLead targets in their direction of travel.\nPress ENTER to try again.", run.score, run.best, run.level, accuracy), _ => String::new(),
        };
    }
    for mut n in &mut nodes.p2() {
        n.width = percent(run.kills as f32 / difficulty(run.level).quota as f32 * 100.0);
    }
    let (cam, transform) = *camera;
    for mut node in &mut nodes.p1() {
        node.display = if run.phase == Phase::Playing {
            Display::Flex
        } else {
            Display::None
        };
        if let Ok(p) = cam.world_to_viewport(transform, Vec3::new(run.aim.x, run.aim.y, TARGET_Z)) {
            node.left = px(p.x - 15.0);
            node.top = px(p.y - 15.0);
        }
    }
    for mut color in &mut damage {
        color.0 = Color::srgba(1.0, 0.07, 0.1, run.flash * 0.35);
    }
}

fn smoke_test(
    mut smoke: ResMut<Smoke>,
    mut commands: Commands,
    mut run: ResMut<Run>,
    mut exit: MessageWriter<AppExit>,
) {
    if !smoke.enabled {
        return;
    }
    smoke.frames += 1;
    if smoke.frames == 60 {
        run.phase = Phase::Playing;
    }
    if smoke.frames == 600 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("/tmp/neon-range-smoke.png"));
    }
    if smoke.frames == 660 {
        info!("SMOKE OK: rendered scene and gameplay for 660 frames");
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                0.02,
            )))
            .insert_resource(Run {
                phase: Phase::Playing,
                spawn: 100.0,
                ..default()
            })
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(Smoke::default())
            .insert_resource(Art {
                cube: default(),
                sphere: default(),
                ring: default(),
                metal: default(),
                cyan: default(),
                orange: default(),
                pink: default(),
            })
            .add_systems(Update, (controls, simulate, effects).chain());
        app.update();
        app
    }
    fn target(app: &mut App, x: f32) -> Entity {
        app.world_mut()
            .spawn((
                Transient,
                Target {
                    velocity: 0.0,
                    base_y: 4.0,
                    age: 0.0,
                    radius: 0.82,
                },
                Transform::from_xyz(x, 4.0, TARGET_Z),
            ))
            .id()
    }
    fn bullet(app: &mut App) {
        app.world_mut().spawn((
            Transient,
            Bullet {
                velocity: Vec3::NEG_Z * 65.0,
                ttl: 0.8,
            },
            Transform::from_xyz(0.0, 4.0, TARGET_Z + 0.5),
        ));
    }
    #[test]
    fn hit_awards_points_and_despawns_target_once() {
        let mut app = app();
        let entity = target(&mut app, 0.0);
        bullet(&mut app);
        bullet(&mut app);
        app.update();
        let run = app.world().resource::<Run>();
        assert_eq!((run.score, run.hits, run.kills), (100, 1, 1));
        assert!(app.world().get_entity(entity).is_err());
    }
    #[test]
    fn quota_advances_level_repairs_and_clears_wave() {
        let mut app = app();
        {
            let mut run = app.world_mut().resource_mut::<Run>();
            run.kills = difficulty(1).quota - 1;
            run.lives = 3;
        }
        target(&mut app, 0.0);
        let other = target(&mut app, 5.0);
        bullet(&mut app);
        app.update();
        let run = app.world().resource::<Run>();
        assert_eq!((run.level, run.kills, run.lives), (2, 0, 4));
        assert!(app.world().get_entity(other).is_err());
    }
    #[test]
    fn escape_ends_run_and_restart_resets_entities() {
        let mut app = app();
        {
            let mut run = app.world_mut().resource_mut::<Run>();
            run.lives = 1;
            run.score = 500;
        }
        target(&mut app, 14.0);
        app.update();
        assert!(app.world().resource::<Run>().phase == Phase::Over);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();
        let run = app.world().resource::<Run>();
        assert!(run.phase == Phase::Playing);
        assert_eq!((run.lives, run.score, run.best), (5, 0, 500));
    }
    #[test]
    fn pause_freezes_projectiles_and_spawn_clock() {
        let mut app = app();
        bullet(&mut app);
        app.world_mut().resource_mut::<Run>().phase = Phase::Paused;
        let before = app.world().resource::<Run>().spawn;
        app.update();
        assert_eq!(app.world().resource::<Run>().spawn, before);
        let mut query = app.world_mut().query::<&Bullet>();
        assert_eq!(query.single(app.world()).unwrap().ttl, 0.8);
    }

    #[test]
    fn keyboard_aims_and_holding_fire_obeys_cooldown() {
        let mut app = app();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ArrowRight);
            keys.press(KeyCode::Space);
        }
        app.update();
        assert!(app.world().resource::<Run>().aim.x > 0.0);
        assert_eq!(app.world().resource::<Run>().shots, 1);
        app.update();
        assert_eq!(app.world().resource::<Run>().shots, 1);
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(app.world().resource::<Run>().shots, 2);
    }
}
