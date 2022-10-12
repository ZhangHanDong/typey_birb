use rand::prelude::*;
use std::ops::Range;

// 上下障碍物之间空隙的大小规格
#[derive(Debug)]
enum NextGapKind {
    VerySmall,
    Small,
    Medium,
    Large,
    VeryLarge,
}
impl NextGapKind {
    fn to_range(&self) -> Range<f32> {
        match self {
            NextGapKind::VerySmall => 0.1..0.2,
            NextGapKind::Small => 0.2..0.3,
            NextGapKind::Medium => 0.3..0.4,
            NextGapKind::Large => 0.4..0.6,
            NextGapKind::VeryLarge => 0.6..1.0,
        }
    }
}
pub struct NextGapBag {
    rng: StdRng, // 使用 rand 的 RNG(随机数发生器)
    index: usize,
    range: Range<f32>,
    previous_value: f32,
    contents: Vec<NextGapKind>,
}
impl NextGapBag {
    pub fn new(range: Range<f32>, initial_value: f32) -> Self {
        let mut rng = StdRng::from_entropy(); // 创建新的随机种子

        let mut contents = vec![
            NextGapKind::VerySmall,
            NextGapKind::Small,
            NextGapKind::Small,
            NextGapKind::Medium,
            NextGapKind::Medium,
            NextGapKind::Large,
            NextGapKind::Large,
            NextGapKind::VeryLarge,
        ];

        contents.shuffle(&mut rng); // 随机获取 gap 的大小

        // ease them into it

        while contents
            .iter()
            .take(2)
            .any(|k| matches!(k, NextGapKind::Large | NextGapKind::VeryLarge))
        {
            contents.shuffle(&mut rng);
        }

        Self {
            rng,
            range,
            previous_value: initial_value,
            index: 0,
            contents,
        }
    }
}

// 实现一个随机获取gap的迭代器
impl Iterator for NextGapBag {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.contents.len() {
            self.index = 0;
            self.contents.shuffle(&mut self.rng);
        }

        let kind = self.contents.get(self.index).unwrap();
        let kind_range = kind.to_range();

        let magnitude = self.range.end - self.range.start;

        let scaled_range = (kind_range.start * magnitude)..(kind_range.end * magnitude);

        let down_min = (self.previous_value - scaled_range.end).max(self.range.start);
        let down_max = (self.previous_value - scaled_range.start).max(self.range.start);
        let down = down_min..down_max;

        let up_min = (self.previous_value + scaled_range.start).min(self.range.end);
        let up_max = (self.previous_value + scaled_range.end).min(self.range.end);
        let up = up_min..up_max;

        let val = match (up.is_empty(), down.is_empty()) {
            (false, true) => self.rng.gen_range(up),
            (true, false) => self.rng.gen_range(down),
            (false, false) => {
                if self.rng.gen() {
                    self.rng.gen_range(up)
                } else {
                    self.rng.gen_range(down)
                }
            }
            (true, true) => {
                if self.rng.gen() {
                    up.start
                } else {
                    down.start
                }
            }
        };

        self.previous_value = val;

        self.index += 1;

        Some(val)
    }
}
