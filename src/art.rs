use crate::care::Mood;

// Full-size portraits: expressions change without moving the body or ears.
pub fn portrait(species: &str, mood: Mood) -> String {
    let eye = match mood {
        Mood::Happy => "^",
        Mood::Content => "o",
        Mood::Sleepy => "-",
        Mood::Hungry => "O",
        Mood::VeryHungry => ";",
    };
    let template = match species {
        "dog" => {
            r"
       /\           /\
      /  \_________/  \
     /  /           \  \
    (  (   {e}     {e}   )  )
     \_/      Y      \_/
       \    {mouth}    /
        '.___{tongue}___.'
        /   [===]   \    __
       /  /       \  \  /  )
      (  (         )  )/  /
       \__\_______/__/___/
        (___)   (___)"
        }
        "bird" => {
            r"
              /\ /\
          .--'  V  '--.
         /             \
        |   ({e})   ({e})   |
        |      {mouth}      |
         \  .-------.  /
        / \/         \/ \
       /  /           \  \
       \_/    .---.    \_/
          '._/     \_.'
             ||   ||
        ====\||/ \||/===="
        }
        "lizard" => {
            r"
                 __    __
                / {e}\__/{e} \
               (   {upper}   )
                \ {mouth} /
          __     '------'
         /  \    /      \
        / /\ \__/  /  \  \__
       / /  \_____/    \____>-
      / /        /  /\  \
     ( (________/  /  \  \__
      '----------'     '---->-
                    ~ ~ ~"
        }
        _ => {
            r"
         /\           /\
        /  '._______.'  \
       /                 \
      |   ({e})      ({e})   |
     =|        Y         |=
      |     {mouth}      |
       '._            _.'
          /'--------'\     __
         /  /      \  \   /  )
        (  (        )  ) /  /
         \__\______/__/.'  /
          (___) (___)____.'"
        }
    };
    let mouth = match (species, mood) {
        ("dog", Mood::Happy) => r"\___/",
        ("dog", Mood::Content) => " _|_ ",
        ("dog", Mood::Sleepy) => " ~~~ ",
        ("dog", Mood::Hungry) => " (o) ",
        ("dog", Mood::VeryHungry) => r"/---\",
        ("bird", Mood::Happy) => " V ",
        ("bird", Mood::Content | Mood::Sleepy) => " v ",
        ("bird", Mood::Hungry | Mood::VeryHungry) => "< >",
        ("lizard", Mood::Happy) => r" \__/ ",
        ("lizard", Mood::Content) => " .__. ",
        ("lizard", Mood::Sleepy) => " ~~~~ ",
        ("lizard", Mood::Hungry) => " (__) ",
        ("lizard", Mood::VeryHungry) => r"/    \",
        (_, Mood::Happy) => r"\__|__/",
        (_, Mood::Content) => "  _|_  ",
        (_, Mood::Sleepy) => "  ~~~  ",
        (_, Mood::Hungry) => "  (o)  ",
        (_, Mood::VeryHungry) => r" /---\ ",
    };
    let upper = match mood {
        Mood::Happy | Mood::Hungry => " __ ",
        Mood::VeryHungry => ".--.",
        _ => "    ",
    };
    let portrait = template
        .replace("{e}", eye)
        .replace("{mouth}", mouth)
        .replace("{upper}", upper)
        .replace("{tongue}", if mood == Mood::Happy { " U " } else { "   " });
    if mood == Mood::Sleepy {
        portrait
            .lines()
            .enumerate()
            .map(|(i, line)| {
                if i == 1 {
                    format!("{line}      z")
                } else if i == 2 {
                    format!("{line}   z")
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        portrait
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expressions_preserve_bodies_and_have_distinct_faces() {
        for species in ["cat", "dog", "bird", "lizard"] {
            let portraits: Vec<_> = [
                Mood::Happy,
                Mood::Content,
                Mood::Sleepy,
                Mood::Hungry,
                Mood::VeryHungry,
            ]
            .into_iter()
            .map(|mood| portrait(species, mood))
            .collect();
            for (i, art) in portraits.iter().enumerate() {
                assert!(!art.contains('{'));
                assert!(art.is_ascii());
                assert_eq!(art.lines().count(), 13);
                assert_eq!(
                    art.lines().skip(8).collect::<Vec<_>>(),
                    portraits[0].lines().skip(8).collect::<Vec<_>>()
                );
                for other in &portraits[i + 1..] {
                    assert_ne!(art, other);
                }
            }
        }
    }
}
