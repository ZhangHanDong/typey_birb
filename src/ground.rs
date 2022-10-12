use std::ops::Range;

use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use rand::{thread_rng, Rng};

use crate::{AppState, Speed};

pub const GROUND_LENGTH: f32 = 60.;
const GROUND_WIDTH: f32 = 40.;
const GROUND_VERTICES_X: u32 = 30;
const GROUND_VERTICES_Z: u32 = 20;

// 设置游戏背景组件
#[derive(Component)]
pub struct Ground;


// 定义 GroundBundle 类型，用于在后面创建Ground组件的实体
// 这里使用 pbr 渲染：
// PBR（Physically Based Rendering），基于物理的渲染。
// 它是利用真实世界的原理和理论，通过各种数学方法推导或简化或模拟出一系列渲染方程，并依赖计算机硬件和图形API渲染出拟真画面的技术
// 基于现阶段的知识水平和硬件水平，还不能渲染跟真实世界完全一致的效果，只能一定程序上模拟接近真实世界的渲染画面，
// 故而叫基于物理的渲染（Physically Based Rendering），而非物理渲染（Physical Rendering）
// 参考资料： https://www.cnblogs.com/timlly/p/10631718.html#211-pbr%E6%A6%82%E5%BF%B5
#[derive(Bundle)]
pub struct GroundBundle {
    #[bundle]
    pbr: PbrBundle, // Pbr 渲染 bundle
    ground: Ground,
}

impl GroundBundle {
    pub fn new(
        x: f32,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) -> GroundBundle {
        Self {
            pbr: PbrBundle {
                mesh: meshes.add(ground_mesh(
                    Vec2::new(GROUND_LENGTH, GROUND_WIDTH),
                    UVec2::new(GROUND_VERTICES_X, GROUND_VERTICES_Z),
                )),
                transform: Transform::from_xyz(x, 0.1, 0.),
                material: materials.add(Color::rgb(0.63, 0.96, 0.26).into()),
                ..Default::default()
            },
            ground: Ground,
        }
    }
}

// 定义 Gound插件
pub struct GroundPlugin;

impl Plugin for GroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            // 在 AppState::Playing 状态更新的时候可能的行为：
            // 移动背景，并不断生成新的背景
            SystemSet::on_update(AppState::Playing)
                .with_system(ground_movement.label("ground_movement"))
                .with_system(spawn_ground.after("ground_movement")),
        )
        .add_system_set(SystemSet::on_exit(AppState::Loading).with_system(setup));
    }
}

// 移动背景
fn ground_movement(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform), With<Ground>>,
    time: Res<Time>,
    speed: Res<Speed>,
) {
    // 背景平移增量：按时间增量和当前速度计算
    let delta = time.delta_seconds() * speed.current;

    for (entity, mut transform) in query.iter_mut() {
        // 背景平移
        transform.translation.x -= delta;
        // 如果平移超出范围则消除相关实体
        if transform.translation.x < -60. {
            commands.entity(entity).despawn_recursive();
        }
    }
}

// 生成 ground 
fn spawn_ground(
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    query: Query<&Transform, With<Ground>>,
) {
    // keep two ground chunks alive at all times

    if query.iter().count() >= 2 {
        return;
    }

    let max_x = query
        .iter()
        .max_by(|a, b| a.translation.x.partial_cmp(&b.translation.x).unwrap())
        .unwrap()
        .translation
        .x;
    // 创建实体
    commands.spawn_bundle(GroundBundle::new(max_x + GROUND_LENGTH, meshes, materials));
}

// 初始化ground
fn setup(
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn_bundle(GroundBundle::new(0., meshes, materials));
}

// 绘制背景网格
pub fn ground_mesh(size: Vec2, num_vertices: UVec2) -> Mesh {
    let num_quads = num_vertices - UVec2::splat(1);
    let offset = size / -2.;

    let h_range: Range<f32> = -0.1..0.1;

    let mut rng = thread_rng();

    let mut positions = vec![];
    let mut normals = vec![];
    let mut uvs = vec![];
    let mut indices = vec![];

    for x in 0..num_vertices.x {
        for z in 0..num_vertices.y {
            let h = if x == 0 || x == num_vertices.x - 1 {
                0.0
            } else {
                rng.gen_range(h_range.clone())
            };

            positions.push([
                offset.x + x as f32 / num_quads.x as f32 * size.x,
                h,
                offset.y + z as f32 / num_quads.y as f32 * size.y,
            ]);
            normals.push([0., 1., 0.]);
            uvs.push([0., 0.]);
        }
    }

    for x in 0..num_quads.x {
        for z in 0..num_quads.y {
            let i = x * num_vertices.y + z;

            indices.extend_from_slice(&[
                i,
                i + 1,
                i + num_vertices.y,
                i + num_vertices.y,
                i + 1,
                i + num_vertices.y + 1,
            ]);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
    mesh
}
