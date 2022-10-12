use bevy::{prelude::*, utils::HashSet};
use rand::prelude::*;

// 输入plugin
pub struct TypingPlugin;

// 单词列表
pub struct WordList {
    words: Vec<String>,
    index: usize,
}
// 从 crate::words::WORDS 里随机获取单词
impl Default for WordList {
    fn default() -> Self {
        let mut words = crate::words::WORDS
            .lines()
            .map(|w| w.to_owned())
            .filter(|w| w.chars().count() > 0)
            .collect::<Vec<_>>();
        words.shuffle(&mut thread_rng());
        Self { words, index: 0 }
    }
}

impl WordList {
    // 找到下一个单词
    pub fn find_next_word(&mut self, not: &HashSet<char>) -> String {
        loop {
            let next = self.advance_word();
            if next.chars().all(|c| !not.contains(&c)) {
                return next;
            }
        }
    }

    fn advance_word(&mut self) -> String {
        self.index += 1;
        if self.index >= self.words.len() {
            self.words.shuffle(&mut thread_rng());
            self.index = 0;
        }
        self.words[self.index].clone()
    }
}

#[derive(Component)]
pub struct TypingTarget {
    pub letter_actions: Vec<crate::Action>,
    pub word_actions: Vec<crate::Action>,
    pub index: usize,
    pub word: String,
}

impl TypingTarget {
    pub fn new(word: String, actions: Vec<crate::Action>) -> Self {
        Self {
            letter_actions: actions,
            word_actions: vec![],
            index: 0,
            word,
        }
    }
    pub fn new_whole(word: String, actions: Vec<crate::Action>) -> Self {
        Self {
            word_actions: actions,
            letter_actions: vec![],
            index: 0,
            word,
        }
    }
    pub fn current_char(&self) -> Option<char> {
        self.word.chars().nth(self.index)
    }
    pub fn advance_char(&mut self) -> Option<char> {
        self.index += 1;
        self.current_char()
    }
    pub fn replace(&mut self, new: String) {
        self.word = new;
        self.index = 0;
    }
}

impl Plugin for TypingPlugin {
    fn build(&self, app: &mut App) {
        // 初始化单词资源
        app.init_resource::<WordList>()
            .add_system(new_words)
            .add_system(keyboard);
    }
}

// 获取新的单词
fn new_words(
    mut events: EventReader<crate::Action>,
    mut query: Query<(Entity, &mut TypingTarget)>,
    mut wordlist: ResMut<WordList>,
) {
    for e in events.iter() {
        if let crate::Action::NewWord(entity) = e {
            // build a list of characters to avoid for the next word,
            // skipping the word we're replacing.
            let not: HashSet<char> = query
                .iter()
                .filter(|(e, _)| e != entity)
                .flat_map(|(_, t)| t.word.chars())
                .collect();

            if let Ok((_, mut target)) = query.get_mut(*entity) {
                let next = wordlist.find_next_word(&not);
                target.replace(next);
            }
        }
    }
}

// 键盘输入
fn keyboard(
    // EventReader 接收输入字符
    mut char_input_events: EventReader<ReceivedCharacter>,
    mut query: Query<(Entity, &mut TypingTarget)>,
    mut events: EventWriter<crate::Action>,
) {
    // 判断收到的字符是否匹配显示单词的每个字符
    for event in char_input_events.iter() {
        let mut ok = false;

        for (entity, mut target) in query.iter_mut() {
            if let Some(next) = target.current_char() {
                if next == event.char {
                    for action in target.letter_actions.iter() {
                        events.send(action.clone());
                    }

                    if target.advance_char().is_none() {
                        events.send(crate::Action::NewWord(entity));

                        for action in target.word_actions.iter() {
                            events.send(action.clone());
                        }
                    }

                    ok = true;
                }
            }
        }

        if !ok {
            events.send(crate::Action::BadFlap);
        }
    }
}
