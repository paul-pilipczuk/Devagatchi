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

pub const GUIDE: &str = "CARE SCHEDULE (time since adoption or the most recent meal)
  Both modes: happy for the first hour, then content.
  Cozy: content until 72 hours (3 days), then sleepy. Never hungry.
  Traditional: content until 24 hours (1 day), hungry until 48 hours (2 days),
               then very hungry. No death or permanent harm.
  A manual feed or sync with new commits resets the timer to happy.
  Bigger meals do not extend the timer. A sync with no new commits does not reset it.
  Changing modes applies the new schedule immediately without resetting the timer.
  Time passes while the app is closed. Faces and status update when you run a command;
  there is no background watcher. Status alone does not feed your pet.";

pub fn schedule(mode: &str, elapsed: u64) -> (Mood, Option<(Mood, u64)>) {
    if elapsed < 3600 {
        (Mood::Happy, Some((Mood::Content, 3600 - elapsed)))
    } else if mode == "cozy" {
        if elapsed < 72 * 3600 {
            (Mood::Content, Some((Mood::Sleepy, 72 * 3600 - elapsed)))
        } else {
            (Mood::Sleepy, None)
        }
    } else if elapsed < 24 * 3600 {
        (Mood::Content, Some((Mood::Hungry, 24 * 3600 - elapsed)))
    } else if elapsed < 48 * 3600 {
        (Mood::Hungry, Some((Mood::VeryHungry, 48 * 3600 - elapsed)))
    } else {
        (Mood::VeryHungry, None)
    }
}

pub fn duration(seconds: u64) -> String {
    if seconds < 60 {
        return "less than a minute".into();
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        format!("{minutes}m")
    } else if minutes < 24 * 60 {
        format!("{}h {}m", minutes / 60, minutes % 60)
    } else {
        format!(
            "{}d {}h {}m",
            minutes / (24 * 60),
            (minutes / 60) % 24,
            minutes % 60
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_transition_boundaries() {
        for mode in ["cozy", "traditional"] {
            assert_eq!(
                schedule(mode, 0),
                (Mood::Happy, Some((Mood::Content, 3600)))
            );
            assert_eq!(
                schedule(mode, 3599),
                (Mood::Happy, Some((Mood::Content, 1)))
            );
            assert_eq!(schedule(mode, 3600).0, Mood::Content);
        }
        for (mode, boundary, before, after) in [
            ("cozy", 72 * 3600, Mood::Content, Mood::Sleepy),
            ("traditional", 24 * 3600, Mood::Content, Mood::Hungry),
            ("traditional", 48 * 3600, Mood::Hungry, Mood::VeryHungry),
        ] {
            assert_eq!(schedule(mode, boundary - 1), (before, Some((after, 1))));
            assert_eq!(schedule(mode, boundary).0, after);
            assert_eq!(schedule(mode, boundary + 1).0, after);
        }
        assert_eq!(schedule("cozy", u64::MAX), (Mood::Sleepy, None));
        assert_eq!(schedule("traditional", u64::MAX), (Mood::VeryHungry, None));
    }
    #[test]
    fn readable_durations() {
        assert_eq!(duration(59), "less than a minute");
        assert_eq!(duration(60), "1m");
        assert_eq!(duration(3600), "1h 0m");
        assert_eq!(duration(72 * 3600), "3d 0h 0m");
    }
}
