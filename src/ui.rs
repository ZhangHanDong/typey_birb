use crate::{
    typing::{TypingTarget, WordList},
    Action, AppState, FontAssets, GltfAssets, Score,
};
use bevy::{prelude::*, utils::HashSet};

// 定义 ui 插件
pub struct UiPlugin;

#[derive(Component)]
struct ScoreText;
#[derive(Component)]
struct StartScreen;
#[derive(Component)]
struct EndScreen;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // We need the font to have been loaded for this to work.
        app.add_system(update_targets)// 增加 update_targets system
            .add_system(update_score) 
            // 在进入 AppState::EndScreen 状态时，执行 death_screen
            .add_system_set(SystemSet::on_enter(AppState::EndScreen).with_system(death_screen))
            // 在结束 AppState::Loading 状态时，执行 setup
            .add_system_set(SystemSet::on_exit(AppState::Loading).with_system(setup))
            // 在进入AppState::StartScreen 状态时，执行 start_screen
            .add_system_set(SystemSet::on_enter(AppState::StartScreen).with_system(start_screen))
            // 在结束 AppState::StartScreen 状态时，执行 despawn_start_screen
            .add_system_set(
                SystemSet::on_exit(AppState::StartScreen).with_system(despawn_start_screen),
            )
            // 在结束 AppState::EndScreen 状态时，执行 despawn_dead_screen
            .add_system_set(
                SystemSet::on_exit(AppState::EndScreen).with_system(despawn_dead_screen),
            );
    }
}

// 递归消除 dead screen时 UI实体
fn despawn_dead_screen(mut commands: Commands, query: Query<Entity, With<EndScreen>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

// 递归消除 start screen时 UI实体
fn despawn_start_screen(mut commands: Commands, query: Query<Entity, With<StartScreen>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

// 使用 bevy_ui 中提供的 NodeBundle 和 TextBundle Widget 来创建 UI 实体
fn start_screen(
    mut commands: Commands,
    gltf_assets: Res<GltfAssets>,
    font_assets: Res<FontAssets>,
) {
    // rival 竞争角色 创建实体

    commands
        .spawn_bundle(SceneBundle {
            scene: gltf_assets.birb_gold.clone(),
            transform: Transform::from_xyz(8.4, 4.0, -0.2)
                .with_scale(Vec3::splat(2.5))
                .with_rotation(Quat::from_euler(EulerRot::XYZ, -0.1, -2.5, -0.8)),
            ..default()
        })
        .insert(StartScreen); // 插入开始屏幕组件

    // text 创建文本组件，使用 NodeBundle 作为容器，基于 Flexbox 布局

    let container = commands
        .spawn_bundle(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                position: UiRect {
                    bottom: Val::Px(0.),
                    right: Val::Px(0.),
                    ..Default::default()
                },
                size: Size::new(Val::Percent(50.0), Val::Percent(70.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::ColumnReverse,
                ..Default::default()
            },
            color: Color::NONE.into(),
            ..Default::default()
        })
        .insert(StartScreen)
        .id();
    // 创建背景 Flexbox 容器
    let bg = commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(70.0), Val::Percent(40.0)),
                align_items: AlignItems::FlexStart,
                justify_content: JustifyContent::SpaceBetween,
                flex_direction: FlexDirection::ColumnReverse,
                padding: UiRect::all(Val::Px(10.0)),
                ..Default::default()
            },
            color: Color::BLACK.into(),
            ..Default::default()
        })
        .id();

    // 创建开始文本 Flexbox item，本游戏是以输入文字 start 开始的
    let starttext = commands
        .spawn_bundle(TextBundle {
            style: Style {
                ..Default::default()
            },
            text: Text {
                sections: vec![TextSection {
                    value: "So you want to join the flock, eh?\nYou'll have to beat me first!\nType the word below when you're ready."
                        .into(),
                    style: TextStyle {
                        font: font_assets.main.clone(),
                        font_size: 40.,
                        color: Color::WHITE,
                    },
                }],
                ..Default::default()
            },
            ..Default::default()
        })
        .id();

    let starttarget = commands
        .spawn_bundle(TextBundle {
            style: Style {
                ..Default::default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "".into(),
                        style: TextStyle {
                            font: font_assets.main.clone(),
                            font_size: 40.,
                            color: Color::GREEN,
                        },
                    },
                    TextSection {
                        value: "START".into(),
                        style: TextStyle {
                            font: font_assets.main.clone(),
                            font_size: 40.,
                            color: Color::rgb_u8(255, 235, 146),
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(TypingTarget::new_whole("start".into(), vec![Action::Start]))
        .id();

    // 创建实体
    commands.entity(container).push_children(&[bg]);
    commands.entity(bg).push_children(&[starttext, starttarget]);
}

// 游戏结束后的屏幕 ui 
fn death_screen(
    mut commands: Commands,
    gltf_assets: Res<GltfAssets>,
    font_assets: Res<FontAssets>,
    score: Res<Score>,
) {
    let death_msg = if score.0 > 1000 {
        "I... wha... wow!\nWhat am I even doing with my life?\nThe flock is yours, if you'll have us!"
    } else if score.0 > 400 {
        "That was a close one!\nWith moves like that, you'll\nfit in well here!"
    } else if score.0 > 200 {
        "Not bad, kid!\nThere may be room for you in the flock\nas an unpaid apprentice."
    } else {
        "Oh wow, ouch!\nToo bad you're stuck at Z = 0.0,\nthe path is a bit clearer a few units over."
    };

    // rival

    commands
        .spawn_bundle(SceneBundle {
            scene: gltf_assets.birb_gold.clone(),
            transform: Transform::from_xyz(8.4, 4.0, -0.2)
                .with_scale(Vec3::splat(2.5))
                .with_rotation(Quat::from_euler(EulerRot::XYZ, -0.1, -2.5, -0.8)),
            ..default()
        })
        .insert(EndScreen);

    // text 创建文本 Flexbox item
    
    let container = commands
        .spawn_bundle(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                position: UiRect {
                    bottom: Val::Px(0.),
                    right: Val::Px(0.),
                    ..Default::default()
                },
                size: Size::new(Val::Percent(50.0), Val::Percent(70.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::ColumnReverse,
                ..Default::default()
            },
            color: Color::NONE.into(),
            ..Default::default()
        })
        .insert(EndScreen)
        .id();
    // 创建 背景 文本 Flexbox item
    let bg = commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(70.0), Val::Percent(40.0)),
                align_items: AlignItems::FlexStart,
                justify_content: JustifyContent::SpaceBetween,
                flex_direction: FlexDirection::ColumnReverse,
                padding: UiRect::all(Val::Px(10.0)),
                ..Default::default()
            },
            color: Color::BLACK.into(),
            ..Default::default()
        })
        .id();
    // 创建文本 Flexbox item
    let deadtext = commands
        .spawn_bundle(TextBundle {
            style: Style {
                ..Default::default()
            },
            text: Text {
                sections: vec![TextSection {
                    value: death_msg.into(),
                    style: TextStyle {
                        font: font_assets.main.clone(),
                        font_size: 40.,
                        color: Color::WHITE,
                    },
                }],
                ..Default::default()
            },
            ..Default::default()
        })
        .id();
    // 创建 重试text Flexbox item
    let retrytext = commands
        .spawn_bundle(TextBundle {
            style: Style {
                ..Default::default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "".into(),
                        style: TextStyle {
                            font: font_assets.main.clone(),
                            font_size: 40.,
                            color: Color::GREEN,
                        },
                    },
                    TextSection {
                        value: "RETRY".into(),
                        style: TextStyle {
                            font: font_assets.main.clone(),
                            font_size: 40.,
                            color: Color::rgb_u8(255, 235, 146),
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(TypingTarget::new_whole("retry".into(), vec![Action::Retry]))
        .id();

    commands.entity(container).push_children(&[bg]);
    commands.entity(bg).push_children(&[deadtext, retrytext]);
}

// 更新分数
fn update_score(mut query: Query<&mut Text, With<ScoreText>>, score: Res<Score>) {
    if !score.is_changed() {
        return;
    }
    for mut text in query.iter_mut() {
        // 查询文本ui 显示分数
        text.sections[1].value = format!("{}", score.0);
    }
}

// 更新目标单词
fn update_targets(
    query: Query<(Entity, &TypingTarget), Changed<TypingTarget>>,
    mut text_query: Query<&mut Text>,
) {
    for (entity, target) in query.iter() {
        if let Ok(mut text) = text_query.get_mut(entity) {
            let parts = target.word.split_at(target.index);

            text.sections[0].value = parts.0.to_uppercase();
            text.sections[1].value = parts.1.to_uppercase();
        }
    }
}

// 初始化上下文本框中显示的单词
fn setup(mut commands: Commands, mut wordlist: ResMut<WordList>, font_assets: Res<FontAssets>) {
    // root node
    let root = commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::SpaceBetween,
                flex_direction: FlexDirection::ColumnReverse,
                ..Default::default()
            },
            color: Color::NONE.into(),
            ..Default::default()
        })
        .id();

    let topbar = commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Px(50.)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect {
                    bottom: Val::Px(5.),
                    ..Default::default()
                },
                ..Default::default()
            },
            color: Color::BLACK.into(),
            ..Default::default()
        })
        .id();

    let mut not: HashSet<char> = "start".chars().collect();
    let topword = wordlist.find_next_word(&not);
    for c in topword.chars() {
        not.insert(c);
    }

    let toptext = commands
        .spawn_bundle(TextBundle {
            style: Style {
                margin: UiRect::all(Val::Px(5.0)),
                ..Default::default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "".into(),
                        style: TextStyle {
                            font: font_assets.main.clone(),
                            font_size: 40.,
                            color: Color::GREEN,
                        },
                    },
                    TextSection {
                        value: topword.clone(),
                        style: TextStyle {
                            font: font_assets.main.clone(),
                            font_size: 40.,
                            color: Color::rgb_u8(255, 235, 146),
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(TypingTarget::new(
            topword,
            vec![Action::BirbUp, Action::IncScore(1)],
        ))
        .id();

    let bottombar = commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Px(50.)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect {
                    bottom: Val::Px(5.),
                    ..Default::default()
                },
                ..Default::default()
            },
            color: Color::BLACK.into(),
            ..Default::default()
        })
        .id();

    let bottomword = wordlist.find_next_word(&not);
    let bottomtext = commands
        .spawn_bundle(TextBundle {
            style: Style {
                margin: UiRect::all(Val::Px(5.0)),
                ..Default::default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "".into(),
                        style: TextStyle {
                            font: font_assets.main.clone(),
                            font_size: 40.,
                            color: Color::GREEN,
                        },
                    },
                    TextSection {
                        value: bottomword.clone(),
                        style: TextStyle {
                            font: font_assets.main.clone(),
                            font_size: 40.,
                            color: Color::rgb_u8(255, 235, 146),
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(TypingTarget::new(
            bottomword,
            vec![Action::BirbDown, Action::IncScore(1)],
        ))
        .id();

    let scoretext = commands
        .spawn_bundle(TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                position: UiRect {
                    top: Val::Px(3.0),
                    left: Val::Px(10.0),
                    ..Default::default()
                },
                padding: UiRect::all(Val::Px(5.0)),
                ..Default::default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "SCORE ".into(),
                        style: TextStyle {
                            font: font_assets.main.clone(),
                            font_size: 40.,
                            color: Color::rgba(0.8, 0.8, 0.8, 1.0),
                        },
                    },
                    TextSection {
                        value: "0".into(),
                        style: TextStyle {
                            font: font_assets.main.clone(),
                            font_size: 40.,
                            color: Color::WHITE,
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(ScoreText)
        .id();

    commands.entity(root).push_children(&[topbar, bottombar]);
    commands.entity(topbar).push_children(&[toptext, scoretext]);
    commands.entity(bottombar).push_children(&[bottomtext]);
}
