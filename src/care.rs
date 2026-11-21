#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    Happy,
    Content,
    Sleepy,
    Hungry,
    VeryHungry,
}
impl Mood {
    pub fn label(self) -> &'static str {
        match self {
            Self::Happy => "happy",
            Self::Content => "content",
            Self::Sleepy => "sleepy",
            Self::Hungry => "hungry",
            Self::VeryHungry => "very hungry",
        }
    }
}
pub const GUIDE: &str = "FOOD & CARE
  Food ranges from 0 to 100. A full bar lasts 72h in cozy or 24h in traditional.
  80-100: happy. 40-79: content. Below 40: sleepy (cozy) or hungry (traditional).
  At zero: cozy rests; traditional is very hungry. No death or lost progress.
  Daily food = min(100, floor(4 * sqrt(eligible added + deleted lines))).
  Each event earns only the increase in that day's total; days use UTC.
  Food above 100 is discarded. Manual lines share the same daily allowance.
  Time passes while closed. Mode changes apply the old rate up to the change.
  Hooks feed after commits; sync catches missed work. Status never feeds.";
pub fn earned(lines: u64) -> u64 {
    // Integer arithmetic: at 625 lines the daily reward is already saturated.
    let scaled = lines.min(625) * 16;
    (0..=100).rev().find(|n| n * n <= scaled).unwrap_or(0)
}
pub fn drain(food: f64, mode: &str, seconds: u64) -> f64 {
    (food - seconds as f64 * 100.0 / period(mode) as f64).max(0.0)
}
pub fn period(mode: &str) -> u64 {
    if mode == "cozy" {
        72 * 3600
    } else {
        24 * 3600
    }
}
pub fn mood(food: f64, mode: &str) -> Mood {
    if food >= 80.0 {
        Mood::Happy
    } else if food >= 40.0 {
        Mood::Content
    } else if mode == "cozy" {
        Mood::Sleepy
    } else if food > 0.0 {
        Mood::Hungry
    } else {
        Mood::VeryHungry
    }
}
pub fn duration(seconds: u64) -> String {
    if seconds < 60 {
        "less than a minute".into()
    } else if seconds < 86400 {
        format!("{}h {}m", seconds / 3600, seconds / 60 % 60)
    } else {
        format!("{}d {}h", seconds / 86400, seconds / 3600 % 24)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rewards_and_decay() {
        for (lines, food) in [
            (0, 0),
            (1, 4),
            (10, 12),
            (25, 20),
            (100, 40),
            (400, 80),
            (625, 100),
            (u64::MAX, 100),
        ] {
            assert_eq!(earned(lines), food);
        }
        assert_eq!(earned(25) + earned(100) - earned(25), earned(100));
        assert_eq!(drain(100., "cozy", 72 * 3600), 0.);
        assert_eq!(drain(100., "traditional", 12 * 3600), 50.);
        assert_eq!(mood(40., "cozy"), Mood::Content);
        assert_eq!(mood(39., "cozy"), Mood::Sleepy);
        assert_eq!(mood(0., "traditional"), Mood::VeryHungry);
    }
}
