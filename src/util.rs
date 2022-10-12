use bevy::render::primitives::Aabb;

// 图示参考：https://developer.mozilla.org/zh-CN/docs/Games/Techniques/3D_collision_detection
// 具体的碰撞检测算法
pub fn collide_aabb(a: &Aabb, b: &Aabb) -> bool {
    // a 的最小值指 左上角xy坐标
    let a_min = a.min();
    // a 的最大值指 右下角xy坐标
    let a_max = a.max();
    let b_min = b.min();
    let b_max = b.max();

    // 只需验证物体A与物体B是否满足如下条件：
    // - 物体A的Y轴方向最小值大于物体B的Y轴方向最大值；
    // - 物体A的X轴方向最小值大于物体B的X轴方向最大值；
    // - 物体B的Y轴方向最小值大于物体A的Y轴方向最大值；
    // - 物体B的X轴方向最小值大于物体A的X轴方向最大值；
    // 若满足上述条件，则证明物体A与物体B并未发生重合
    // 反之，则证明物体A与物体B重合。
    a_max.x > b_min.x
        && a_min.x < b_max.x
        && a_max.y > b_min.y
        && a_min.y < b_max.y
        && a_max.z > b_min.z
        && a_min.z < b_max.z
}
