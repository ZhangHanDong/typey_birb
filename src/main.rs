#![allow(clippy::forget_non_drop)] // https://github.com/bevyengine/bevy/issues/4601

use bevy::{
    audio::AudioSink,
    log::{Level, LogSettings},
    math::Vec3A,
    prelude::*,
    render::primitives::Aabb,
};

// 使用第三方库(bevy 插件) bevy_asset_loader 来管理 Assets
use bevy_asset_loader::prelude::*;

// 使用 bevy_inspector_egui 可以进行可视化调试
#[cfg(feature = "inspector")]
use bevy_inspector_egui::WorldInspectorPlugin;
use luck::NextGapBag;
// 使用 bevy 提供的 `bevy::render::primitives::Aabb` 功能进行碰撞检测
use util::collide_aabb;

// 圆柱体障碍
mod cylinder;
// 游戏背景
mod ground;
// 随机产生圆柱体大小、间隔
mod luck;
// 处理键盘输入的打字模块
mod typing;
// 游戏 UI 界面模块
mod ui;
// 工具模块
mod util;
// 产生打字需要的单词
mod words;


// bevy_asset_loader 插件提供了 `AssetCollection` trait 和 派生宏 
// 对于实现了该trait 的结构体，会自动加载 assets
// Handle 可以看作是被加载的 Assets 的一种“指针”，与 Assets 一一对应
#[derive(AssetCollection)]
struct GltfAssets {
    // asset 属性也是 bevy_asset_loader 提供的
    // 加载 assets/.gld，是一种 GLTF 3D模型导出格式
    // 其中包含 网格、场景、材料、纹理等信息
    #[asset(path = "bevybird_gold.glb#Scene0")]
    birb_gold: Handle<Scene>, // 游戏中 金色的鸟，不由玩家操控，自动飞行，和玩家比速度的，但不会碰撞圆柱
    #[asset(path = "bevybird.glb#Scene0")]
    birb: Handle<Scene>, // 玩家操控的鸟
}

// 同上
// 加载字体资源
#[derive(AssetCollection)]
struct FontAssets {
    #[asset(path = "Amatic-Bold.ttf")]
    main: Handle<Font>,
}

// 同上，加载声音资源
#[derive(AssetCollection)]
struct AudioAssets {
    #[asset(path = "menu.ogg")]
    menu: Handle<AudioSource>,
    #[asset(path = "play.ogg")]
    game: Handle<AudioSource>,
    #[asset(path = "flap.ogg")]
    flap: Handle<AudioSource>,
    #[asset(path = "badflap.ogg")]
    badflap: Handle<AudioSource>,
    #[asset(path = "score.ogg")]
    score: Handle<AudioSource>,
    #[asset(path = "crash.ogg")]
    crash: Handle<AudioSource>,
    #[asset(path = "bump.ogg")]
    bump: Handle<AudioSource>,
}

//  bevy::audio::AudioSink 用于控制声音资源
struct MusicController(Handle<AudioSink>);


// 定义 App 状态
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    Loading, // 正在加载
    StartScreen, // 开始屏幕
    Playing, // 游戏中
    #[cfg(feature = "inspector")]
    Paused, // 暂停，用于调试
    EndScreen, // 结束屏幕
}

// Components

// 定义 Bird 组件，玩家操控的鸟
#[derive(Component)]
struct Birb;
// 定义 竞争的金色 Bird 组件，非玩家操控
#[derive(Component)]
struct Rival;

// 定义目标位置组件
#[derive(Component)]
struct TargetPosition(Vec3); // Vec3 代表 3D 向量

// 定义当前 z 轴旋转角度组件
// 本游戏 3D 模型只需要 x和z 轴变换
#[derive(Component)]
struct CurrentRotationZ(f32);


// 定义鸟的动作
#[derive(Clone, Debug)]
pub enum Action {
    BadFlap, // 碰撞以后停止摆动翅膀
    BirbUp, // 鸟向上飞
    BirbDown, // 鸟向下飞
    NewWord(Entity), // 新的单词出现
    IncScore(u32), // 分数增量
    Start, // 开始
    Retry, // 重试
}

// 障碍物（圆柱体）组件
#[derive(Component)]
struct Obstacle;
// 分数增量计算，用于碰撞检测
#[derive(Component)]
struct ScoreCollider;
// 障碍物碰撞，用于碰撞检测
#[derive(Component)]
struct ObstacleCollider;
// 用于碰撞检测中用于标记未碰撞障碍物
#[derive(Component)]
struct Used;

// Resources
// 资源，乃全局变量
#[derive(Default)]
struct Score(u32); // 分数
#[derive(Default)]
struct DistanceToSpawn(f32); // 生成障碍物之间距离
struct ObstacleSpacing(f32); // 障碍物起始空间距离，默认为 12.0
impl Default for ObstacleSpacing {
    fn default() -> Self {
        Self(12.)
    }
}

// 速度
struct Speed {
    current: f32,
    max: f32,
}
impl Default for Speed {
    fn default() -> Self {
        Self {
            current: 2.,
            max: 4.4,
        }
    }
}
impl Speed {
    fn increase(&mut self, amt: f32) {
        self.current = (self.current + amt).min(self.max);
    }
}

// bird 起始坐标
const BIRB_START_Y: f32 = 3.;
// bird 上下坐标范围
const BIRB_MIN_Y: f32 = 0.9;
const BIRB_MAX_Y: f32 = 6.3;

// 上下障碍物之间空隙大小和坐标范围
const GAP_SIZE: f32 = 2.;
const GAP_START_MIN_Y: f32 = 0.5;
const GAP_START_MAX_Y: f32 = 6.7 - GAP_SIZE;

fn main() {
    let mut app = App::new();
    // app 资源加载状态
    app.add_loading_state(
        LoadingState::new(AppState::Loading)
            .continue_to_state(AppState::StartScreen)
            .with_collection::<GltfAssets>()
            .with_collection::<FontAssets>()
            .with_collection::<AudioAssets>(),
    );

    // 插入窗口描述
    app.insert_resource(WindowDescriptor {
        title: "Typey Birb".into(),
        ..Default::default()
    })
    // 使用 ClearColor 清除颜色缓冲区中像素数据
    // 由于缓冲区中可能保留有上一次绘图遗留下来的图像数据，这些数据会影响本次绘图，因此在绘制新图之前必须将它们清除掉。
    .insert_resource(ClearColor(Color::rgb_u8(177, 214, 222))) 
    // 设置 log
    .insert_resource(LogSettings {
        level: Level::INFO,
        ..Default::default()
    })
    .add_plugins(DefaultPlugins);

    // 用于调试
    #[cfg(feature = "inspector")]
    {
        app.add_plugin(WorldInspectorPlugin::new());
        app.add_system_set(SystemSet::on_update(AppState::Paused).with_system(pause));
        app.add_system_set(SystemSet::on_update(AppState::Playing).with_system(pause));
    }

    // 设置初始化loading状态
    app.add_state(AppState::Loading);

    // 初始化资源：分数、速度、障碍物距离和起始空间
    app.init_resource::<Score>()
        .init_resource::<Speed>()
        .init_resource::<DistanceToSpawn>()
        .init_resource::<ObstacleSpacing>()
        .insert_resource(NextGapBag::new(
            GAP_START_MIN_Y..GAP_START_MAX_Y,
            BIRB_START_Y,
        ))
        .add_event::<Action>();

    // 增加 Plugin ： 打字输入处理、UI和背景
    app.add_plugin(crate::typing::TypingPlugin)
        .add_plugin(crate::ui::UiPlugin)
        .add_plugin(crate::ground::GroundPlugin);

    // 将 SystemSet 增加到 update 阶段（stages）
    // stage 用于 Bevy 底层调度 Schedule, Schedule 以线性顺序来执行其中的各个 stage
    // Stage 执行顺序定义于 https://docs.rs/bevy/latest/bevy/app/struct.App.html#the-stages
    // SystemSet 和 App 状态相关，内部是一个基于栈的状态机
    // 这里告诉 App 在 loading 的状态结束（ `exit`） 时执行一次 setup 
    // setup 在下面定义，用于设置 摄像机
    app.add_system_set(SystemSet::on_exit(AppState::Loading).with_system(setup))
        .add_system_set(
            // 在 StartScreen 开始的时候可能执行的动作
            //  spawn_bird （创建鸟）和 开启屏幕音乐
            SystemSet::on_enter(AppState::StartScreen)
                .with_system(spawn_birb)
                .with_system(start_screen_music),
        )
        .add_system_set(
            // 在 StartScreen 每次更新的时候可能执行的动作
            //  start_screen_movement ，让屏幕动起来
            SystemSet::on_update(AppState::StartScreen).with_system(start_screen_movement),
        )
        .add_system_set(
            // 在 AppState::Playing 状态开始的时候可能执行的动作
            // 生成竞争对手（spawn_rival） 并开启游戏音乐
            SystemSet::on_enter(AppState::Playing)
                .with_system(spawn_rival)
                .with_system(game_music),
        )
        .add_system_set(
            // 在 AppState::Playing 状态 每次更新的时候可能执行的动作
            SystemSet::on_update(AppState::Playing)
                // 移动鸟
                .with_system(movement)
                // 移动竞争对手
                .with_system(rival_movement)
                //  碰撞检测
                .with_system(collision)
                // 移动障碍物（产生小鸟向前飞行的效果）
                .with_system(obstacle_movement)
                // 生成新的障碍物
                .with_system(spawn_obstacle)
                // 更新目标位置
                .with_system(update_target_position)
                // 更新分数
                .with_system(update_score)
                // 播放碰撞失败音乐
                .with_system(bad_flap_sound),
        )
        .add_system_set(
            // 在 AppState::StartScreen 状态每次更新的时候可能执行的动作
            // 执行 start_game 和 bad_flap_sound
            SystemSet::on_update(AppState::StartScreen)
                .with_system(start_game)
                .with_system(bad_flap_sound),
        )
        .add_system_set(
            // 在 AppState::EndScreen 状态更新的时候可能执行的动作
            SystemSet::on_update(AppState::EndScreen)
                // 移动竞争鸟角色
                .with_system(rival_movement)
                // 重试游戏
                .with_system(retry_game)
                // 播放碰撞失败音乐
                .with_system(bad_flap_sound),
        )
        // 在 AppState::EndScreen 状态结束的时候执行 reset
        .add_system_set(SystemSet::on_exit(AppState::EndScreen).with_system(reset))
        .run();
}

// 用于调试
#[cfg(feature = "inspector")]
fn pause(mut keyboard: ResMut<Input<KeyCode>>, mut state: ResMut<State<AppState>>) {
    if keyboard.just_pressed(KeyCode::Escape) {
        match state.current() {
            AppState::Paused => {
                state.set(AppState::Playing).unwrap();
                keyboard.clear();
            }
            AppState::Playing => {
                state.set(AppState::Paused).unwrap();
                keyboard.clear();
            }
            _ => {}
        }
    }
}

// 重置游戏状态
// bevy 中使用 Query 来查询 World 范围内的 实体和组件
fn reset(
    mut commands: Commands,
    // 当前 Query类型参数代表使用 Entity ID 进行查询
    // 并且使用 Or 过滤器判断拥有 Obstacle、Bird、Rival 组件的实体之一
    // Query 等价于 ECS 中的 SQL
    query: Query<Entity, Or<(With<Obstacle>, With<Birb>, With<Rival>)>>,
) {
    commands.insert_resource(Score::default());
    commands.insert_resource(Speed::default());
    commands.insert_resource(DistanceToSpawn::default());
    commands.insert_resource(ObstacleSpacing::default());

    for entity in query.iter() {
        // 将查询到的实体递归销毁
        commands.entity(entity).despawn_recursive();
    }
}


// 定义竞争鸟的移动，不受玩家控制，也不与障碍物碰撞
// 其功能只用来和玩家控制的角色比较速度
fn rival_movement(mut query: Query<&mut Transform, With<Rival>>, time: Res<Time>) {
    let speed = 5.; // 固定速度

    // 让角色在 x 和 y 坐标方向进行平移变换（translation）
    // 表现出来的效果就是该角色往前上上下下往复运动
    for mut transform in query.iter_mut() {
        if transform.translation.x < 3. {
            transform.translation.x += speed * time.delta_seconds();
        }

        let floaty = (time.seconds_since_startup() as f32).sin();
        transform.translation.y = 4. + floaty;
        // 还有一次旋转
        // Quat 是表示四元数，可以搜索「渲染 四元数 旋转」
        transform.rotation = Quat::from_rotation_z((time.seconds_since_startup() as f32).cos() / 4.)
    }
}

// 生成 竞争鸟 实体并插入组件数据
fn spawn_rival(mut commands: Commands, gltf_assets: Res<GltfAssets>) {
    commands
        .spawn_bundle(SceneBundle { // Bundle 可以看作一种模版，通过它可以很容易创建一组使用通用组件的实体
            scene: gltf_assets.birb_gold.clone(),
            transform: Transform::from_xyz(-10., 4., 2.5).with_scale(Vec3::splat(0.25)), // 对模型进行大小缩放
            ..default()
        })
        .insert(CurrentRotationZ(0.))
        .insert(Rival);
}

// 当发生BadFlap事件时播放对应音乐
fn bad_flap_sound(
    audio_assets: Res<AudioAssets>,
    audio: Res<Audio>,
    mut events: EventReader<Action>,
) {
    for e in events.iter() {
        if let Action::BadFlap = e {
            audio.play(audio_assets.badflap.clone());
        }
    }
}

// 游戏音乐
fn game_music(
    mut commands: Commands,
    audio_assets: Res<AudioAssets>,
    audio_sinks: Res<Assets<AudioSink>>,
    audio: Res<Audio>,
    controller: Option<Res<MusicController>>,
) {
    if let Some(controller) = controller {
        if let Some(sink) = audio_sinks.get(&controller.0) {
            sink.pause();
        }
    }
    let handle = audio_sinks
        .get_handle(audio.play_with_settings(audio_assets.game.clone(), PlaybackSettings::LOOP));
    commands.insert_resource(MusicController(handle));
}

// 开屏音乐
fn start_screen_music(
    mut commands: Commands,
    audio_assets: Res<AudioAssets>,
    audio_sinks: Res<Assets<AudioSink>>,
    audio: Res<Audio>,
    controller: Option<Res<MusicController>>,
) {
    if let Some(controller) = controller {
        if let Some(sink) = audio_sinks.get(&controller.0) {
            sink.pause();
        }
    }
    let handle = audio_sinks
        .get_handle(audio.play_with_settings(audio_assets.menu.clone(), PlaybackSettings::LOOP));
    commands.insert_resource(MusicController(handle));
}

// 生成玩家控制的角色
fn spawn_birb(mut commands: Commands, gltf_assets: Res<GltfAssets>) {
    // 位置的三维向量
    let pos = Vec3::new(0., BIRB_START_Y, 0.);

    // Use a slightly more forgiving hitbox than the actual
    // computed Aabb.
    //
    // There's a tradeoff here between head scraping and
    // phantom belly collisions.
    //
    // Let's just live with that and not get too fancy with
    // and a flappy bird clone.

    // 使用  bevy::render::primitives::Aabb 碰撞检测
    // Aabb 碰撞检测是指 轴对齐碰撞箱(Axis-aligned Bounding Box)，是分别从x轴向和y轴向进行碰撞检测的算法
    // 对于需要检测的物体 A和物体 B 我们需要将其用 A盒（box）和 B盒将其包装起来
    // 然后判断A盒和B盒在 x轴向和 y轴向是否发生碰撞，只有在 x 轴向和 y轴向都发生碰撞我们才判断它发生了碰撞。
    // 具体碰撞检测算法见 util.rs
    let aabb = Aabb {
        center: Vec3A::splat(0.),
        half_extents: Vec3A::new(0.2, 0.3, 0.25),
    };

    // 创建 bird 实体
    commands
        .spawn_bundle(SceneBundle {
            scene: gltf_assets.birb.clone(),
            transform: Transform::from_translation(pos).with_scale(Vec3::splat(0.25)),
            ..default()
        })
        // 插入玩家每次控制的目标位置组件
        .insert(TargetPosition(pos))
        // 当前旋转角度为0
        .insert(CurrentRotationZ(0.))
        // 插入aabb碰撞检测组件
        .insert(aabb)
        // 插入 bird 组件
        .insert(Birb);
}

// 碰撞处理
fn collision(
    mut commands: Commands,
    birb_query: Query<(&Aabb, &Transform), With<Birb>>,
    score_collider_query: Query<
        (&Aabb, &GlobalTransform, Entity),
        (With<ScoreCollider>, Without<Used>),
    >,
    obstacle_collider_query: Query<(&Aabb, &GlobalTransform), With<ObstacleCollider>>,
    mut score: ResMut<Score>,
    mut state: ResMut<State<AppState>>,
    audio_assets: Res<AudioAssets>,
    audio: Res<Audio>,
) {
    let (birb, transform) = birb_query.single();
    let mut birb = birb.clone();
    birb.center += Vec3A::from(transform.translation);

    // 累计经过障碍物且未碰撞次数的分数
    for (score_aabb, transform, entity) in score_collider_query.iter() {
        let mut score_aabb = score_aabb.clone();
        score_aabb.center += Vec3A::from(transform.translation());

        // 
        if collide_aabb(&score_aabb, &birb) {
            commands.entity(entity).insert(Used);
            score.0 += 2;

            audio.play(audio_assets.score.clone());
        }
    }
    // 处理与障碍物碰撞时的状况
    for (obstacle_aabb, transform) in obstacle_collider_query.iter() {
        let mut obstacle_aabb = obstacle_aabb.clone();
        obstacle_aabb.center += Vec3A::from(transform.translation());

        // 检测到障碍物碰撞时结束屏幕并且播放对应音乐
        if collide_aabb(&obstacle_aabb, &birb) {
            state.set(AppState::EndScreen).unwrap();

            audio.play(audio_assets.crash.clone());

            // it's possible to collide with the pipe and flange simultaneously
            // so we should only react to one game-ending collision.
            break;
        }
    }
}

// 生成障碍物
fn spawn_obstacle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    spacing: Res<ObstacleSpacing>,
    mut distance: ResMut<DistanceToSpawn>,
    mut speed: ResMut<Speed>,
    mut bag: ResMut<NextGapBag>,
) {
    if distance.0 > 0. {
        return;
    }

    // 设定初始距离
    distance.0 = spacing.0;

    speed.increase(0.1);

    // 空隙
    let gap_start = bag.next().unwrap();

    // 圆柱体盖子的高度和半径
    let flange_height = 0.4;
    let flange_radius = 0.8;

    // 底部障碍物高度
    let bottom_height = gap_start;
    // 在网格上增加底部圆柱体
    let bottom_cylinder = meshes.add(
        cylinder::Cylinder {
            radius: 0.75,
            resolution: 16,
            segments: 1,
            height: bottom_height,
        }
        .into(), // 将 Cylinder 转为 Mesh (网格)
    );
    let bottom_y = bottom_height / 2.;

    // 顶部圆柱体相关数据设置
    let top_height = 10. - gap_start - GAP_SIZE;
    let top_cylinder = meshes.add(
        cylinder::Cylinder {
            radius: 0.75,
            resolution: 16,
            segments: 1,
            height: top_height,
        }
        .into(),
    );
    let top_y = gap_start + GAP_SIZE + top_height / 2.;

    let flange = meshes.add(
        cylinder::Cylinder {
            radius: flange_radius,
            resolution: 16,
            segments: 1,
            height: flange_height,
        }
        .into(),
    );
    let bottom_flange_y = gap_start - flange_height / 2.;
    let top_flange_y = gap_start + GAP_SIZE + flange_height / 2.;

    // 上下圆柱体中间空隙
    let middle: Mesh = shape::Box {
        min_x: -0.1,
        max_x: 1.0,
        min_y: gap_start,
        max_y: gap_start + GAP_SIZE,
        min_z: -0.5,
        max_z: 0.5,
    }
    .into();

    // 生成圆柱体实体
    // Bevy 支持通过 Parent 和 Children 创建逻辑层次结构
    // 创建四个父圆柱实体，用于生成随着小鸟移动而不断出现的子实体
    commands
        .spawn_bundle((
            Transform::from_xyz(38., 0., 0.),
            GlobalTransform::default(),
            Visibility::default(), // 可见性
            ComputedVisibility::default(),
        ))
        .with_children(|parent| {
            // 创建底部圆柱体
            parent
                .spawn()
                // 插入 Pbr 物理渲染 bundle
                .insert_bundle(PbrBundle {
                    transform: Transform::from_xyz(0., bottom_y, 0.),
                    mesh: bottom_cylinder,
                    material: materials.add(Color::GREEN.into()),
                    ..Default::default()
                })
                .insert(ObstacleCollider); // 插入碰撞检测组件
            // 创建底部圆柱体的盖子
            parent
                .spawn()
                .insert_bundle(PbrBundle {
                    transform: Transform::from_xyz(0., bottom_flange_y, 0.),
                    mesh: flange.clone(),
                    material: materials.add(Color::GREEN.into()),
                    ..Default::default()
                })
                .insert(ObstacleCollider);

            // 创建顶部圆柱体
            parent
                .spawn()
                .insert_bundle(PbrBundle {
                    transform: Transform::from_xyz(0., top_y, 0.),
                    mesh: top_cylinder,
                    material: materials.add(Color::GREEN.into()),
                    ..Default::default()
                })
                .insert(ObstacleCollider);
            // 创建底部圆柱体的盖子
            parent
                .spawn()
                .insert_bundle(PbrBundle {
                    transform: Transform::from_xyz(0., top_flange_y, 0.),
                    mesh: flange.clone(),
                    material: materials.add(Color::GREEN.into()),
                    ..Default::default()
                })
                .insert(ObstacleCollider);

            // 创建上下圆柱体中间aabb层用于计算未碰撞的分数
            parent
                .spawn()
                .insert_bundle((Transform::default(), GlobalTransform::default()))
                .insert(middle.compute_aabb().unwrap())
                .insert(ScoreCollider);
        })
        .insert(Obstacle);
}

// 移动障碍物，制造小鸟向前飞的效果
fn obstacle_movement(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform), With<Obstacle>>,
    time: Res<Time>,
    mut distance: ResMut<DistanceToSpawn>,
    speed: Res<Speed>,
) {
    let delta = time.delta_seconds() * speed.current;

    distance.0 -= delta;

    for (entity, mut transform) in query.iter_mut() {
        // 向后平移造成小鸟向前移动错觉
        transform.translation.x -= delta;
        if transform.translation.x < -30. {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn start_screen_movement(mut query: Query<(&mut Transform, &mut TargetPosition)>, time: Res<Time>) {
    let speed = 1.0;
    let magnitude = 0.15;

    for (mut transform, mut target) in query.iter_mut() {
        let floaty = (time.seconds_since_startup() as f32 * speed).sin() * magnitude;
        transform.translation.y = 3. + floaty;
        target.0 = transform.translation;
    }
}

// 玩家操控小鸟移动
fn movement(
    mut query: Query<(&mut Transform, &mut CurrentRotationZ, &TargetPosition)>,
    time: Res<Time>,
) {
    // 固定的速度
    let speed = 2.;
    let rot_speed = 2.;
    let rot_speed_glide = 1.;

    // 计算每次移动的目标位置等信息，详细不表
    for (mut transform, mut rotation, target) in query.iter_mut() {
        let dist = target.0.distance(transform.translation);

        // if we are not moving, seek a neutral rotation
        if dist <= std::f32::EPSILON {
            if rotation.0.abs() <= std::f32::EPSILON {
                continue;
            }

            let delta = time.delta_seconds() * rot_speed_glide;

            if rotation.0 < 0. {
                rotation.0 = (rotation.0 + delta).min(0.);
            } else {
                rotation.0 = (rotation.0 - delta).max(0.);
            };

            transform.rotation = Quat::from_rotation_z(rotation.0);

            continue;
        }

        // otherwise, rotate with the direction of movement

        let dir = target.0 - transform.translation;

        let rot = if dir.y > 0. {
            time.delta_seconds() * rot_speed
        } else {
            time.delta_seconds() * -rot_speed
        };
        rotation.0 = (rotation.0 + rot).clamp(-0.5, 0.5);
        transform.rotation = Quat::from_rotation_z(rotation.0);

        // seek the target position

        let delta = dir.normalize() * time.delta_seconds() * speed;
        if dist < delta.length() {
            transform.translation = target.0;
        } else {
            transform.translation += delta;
        }
    }
}

// 重试游戏
fn retry_game(mut events: EventReader<Action>, mut state: ResMut<State<AppState>>) {
    for e in events.iter() {
        if let Action::Retry = e {
            // 设置游戏App状态为 AppState::StartScreen
            state.set(AppState::StartScreen).unwrap();
        }
    }
}

// 开始游戏
fn start_game(mut events: EventReader<Action>, mut state: ResMut<State<AppState>>) {
    for e in events.iter() {
        if let Action::Start = e {
            state.set(AppState::Playing).unwrap();
        }
    }
}

// 更新分数
fn update_score(mut events: EventReader<Action>, mut score: ResMut<Score>) {
    for e in events.iter() {
        if let Action::IncScore(inc) = e {
            score.0 += inc
        }
    }
}

// 更新玩家操作小鸟的目标位置
fn update_target_position(
    mut events: EventReader<Action>,
    mut query: Query<&mut TargetPosition>,
    audio_assets: Res<AudioAssets>,
    audio: Res<Audio>,
) {
    // 通过事件读取器 EventReader
    // 获取小鸟的状态，然后更新目标位置和播放音乐
    for e in events.iter() {
        match e {
            // 向上
            Action::BirbUp => {
                for mut target in query.iter_mut() {
                    target.0.y += 0.25;
                    if target.0.y > BIRB_MAX_Y {
                        target.0.y = BIRB_MAX_Y;
                        audio.play(audio_assets.bump.clone());
                    } else {
                        audio.play(audio_assets.flap.clone());
                    }
                }
            }
            // 向下
            Action::BirbDown => {
                for mut target in query.iter_mut() {
                    target.0.y -= 0.25;
                    if target.0.y < BIRB_MIN_Y {
                        target.0.y = BIRB_MIN_Y;
                        audio.play(audio_assets.bump.clone());
                    } else {
                        audio.play(audio_assets.flap.clone());
                    }
                }
            }
            _ => {}
        }
    }
}


// 设置3D摄像机
fn setup(mut commands: Commands) {
    // camera
    // 创建3D摄像机实体
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(4.5, 5.8, 11.7).with_rotation(Quat::from_rotation_x(-0.211)),
        ..Default::default()
    });

    // directional 'sun' light
    // 设置光源
    const HALF_SIZE: f32 = 40.0;
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            // Configure the projection to better fit the scene
            // 配置阴影投影
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..Default::default()
            },
            shadows_enabled: true, // 可以有阴影
            illuminance: 5000., // 光照强度
            ..Default::default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4 / 2.)
                * Quat::from_rotation_y(std::f32::consts::PI / 8.),
            ..Default::default()
        },
        ..Default::default()
    });
}
