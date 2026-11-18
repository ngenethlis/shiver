//! Picking a line to show.

use std::collections::HashMap;
use std::path::PathBuf;

use rand::Rng;
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;

use crate::chorus::Chorus;
use crate::context::{self, Context};
use crate::pack::{Line, Voice};

/// Which lines may be picked. Empty fields match everything.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    /// Only lines from this voice key.
    pub voice: Option<String>,
    /// Only lines with this tag. An explicit context tag is allowed at any time.
    pub tag: Option<String>,
    /// The time-of-day context: restricts and boosts context-tagged lines.
    pub context: Context,
}

impl Filter {
    /// Whether a line's context tags allow it now: it has none, or one of them
    /// is active or explicitly asked for.
    fn context_allows(&self, line: &Line) -> bool {
        let mut tags = line
            .tags
            .iter()
            .filter(|t| context::is_context_tag(t))
            .peekable();
        tags.peek().is_none()
            || tags.any(|t| self.context.is_active(t) || self.tag.as_ref() == Some(t))
    }

    /// The line's weight, boosted if it carries an active context tag.
    fn weight(&self, line: &Line) -> f64 {
        if line.tags.iter().any(|t| self.context.is_active(t)) {
            line.weight * self.context.boost
        } else {
            line.weight
        }
    }
}

/// A line that was shown before, as recorded in the history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    /// The line's id ([`Line::id`]).
    pub id: u64,
    /// The voice that said it.
    pub voice: String,
}

/// A picked line, ready to show.
#[derive(Debug, Clone, PartialEq)]
pub struct Interjection {
    /// The line's id ([`Line::id`]), for recording it in the history.
    pub id: u64,
    /// The voice saying the line.
    pub voice: Voice,
    /// The line's text.
    pub text: String,
    /// Sound to play: the line's own, else the voice's.
    pub sound: Option<PathBuf>,
    /// Icon to show: the voice's.
    pub icon: Option<PathBuf>,
}

impl Chorus {
    /// Picks a random line matching `filter`, weighted by each line's weight.
    ///
    /// Returns `None` if no line matches.
    pub fn pick<R: Rng + ?Sized>(&self, filter: &Filter, rng: &mut R) -> Option<Interjection> {
        self.pick_with_history(filter, &[], rng)
    }

    /// Like [`Chorus::pick`], but avoids what was shown recently.
    ///
    /// `history` is the recent picks, oldest first. Lines not in the history
    /// are preferred; if every matching line is in it, the half shown longest
    /// ago is used. Unless `filter` names a voice, the last pick's voice is
    /// skipped when another voice has a line available.
    pub fn pick_with_history<R: Rng + ?Sized>(
        &self,
        filter: &Filter,
        history: &[Seen],
        rng: &mut R,
    ) -> Option<Interjection> {
        let candidates: Vec<&Line> = self
            .lines
            .iter()
            .filter(|l| filter.voice.as_ref().is_none_or(|v| &l.voice == v))
            .filter(|l| filter.tag.as_ref().is_none_or(|t| l.tags.contains(t)))
            .filter(|l| filter.context_allows(l))
            .collect();
        if candidates.is_empty() {
            return None;
        }

        // Position of each id's most recent showing.
        let last_shown: HashMap<u64, usize> =
            history.iter().enumerate().map(|(i, s)| (s.id, i)).collect();
        let fresh: Vec<&Line> = candidates
            .iter()
            .copied()
            .filter(|l| !last_shown.contains_key(&l.id()))
            .collect();
        let mut pool = if fresh.is_empty() {
            let mut oldest_first = candidates;
            oldest_first.sort_by_key(|l| last_shown.get(&l.id()).copied());
            oldest_first.truncate(oldest_first.len().div_ceil(2));
            oldest_first
        } else {
            fresh
        };

        if filter.voice.is_none()
            && let Some(last) = history.last()
            && pool.iter().any(|l| l.voice != last.voice)
        {
            pool.retain(|l| l.voice != last.voice);
        }

        // Weights are validated at load time (finite, > 0) and the pool isn't
        // empty, so this can't fail.
        let index = WeightedIndex::new(pool.iter().map(|l| filter.weight(l))).ok()?;
        let line = pool[index.sample(rng)];
        let voice = self.voices.get(&line.voice)?.clone();

        Some(Interjection {
            id: line.id(),
            text: line.text.clone(),
            sound: line.sound.clone().or_else(|| voice.sound.clone()),
            icon: voice.icon.clone(),
            voice,
        })
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::*;
    use crate::test_util::Packs;

    /// Two voices; `drift` has two lines (one tagged `dusk`), `ember` has one.
    fn sample_chorus() -> (Packs, Chorus) {
        let packs = Packs::new();
        packs.add(
            "p",
            "[voices.drift]\ndisplay = \"DRIFT\"\ncolor = \"#336699\"\n\
             [voices.ember]\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"Lights off soon.\"\ntags = [\"dusk\"]\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"Look out the window.\"\ntags = [\"idle\"]\n\
             [[lines]]\nvoice = \"ember\"\ntext = \"Still awake?\"\ntags = [\"dusk\"]\n",
        );
        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        (packs, chorus)
    }

    fn filter(voice: Option<&str>, tag: Option<&str>) -> Filter {
        Filter {
            voice: voice.map(Into::into),
            tag: tag.map(Into::into),
            context: Context::none(),
        }
    }

    /// Every text that `filter` can produce over many seeded picks.
    fn texts(chorus: &Chorus, f: &Filter) -> Vec<String> {
        let mut rng = StdRng::seed_from_u64(7);
        let mut seen: Vec<String> = (0..200)
            .filter_map(|_| chorus.pick(f, &mut rng))
            .map(|i| i.text)
            .collect();
        seen.sort();
        seen.dedup();
        seen
    }

    #[test]
    fn filters_by_voice() {
        let (_p, chorus) = sample_chorus();
        assert_eq!(
            texts(&chorus, &filter(Some("drift"), None)),
            ["Lights off soon.", "Look out the window."]
        );
    }

    #[test]
    fn filters_by_tag() {
        let (_p, chorus) = sample_chorus();
        assert_eq!(
            texts(&chorus, &filter(None, Some("dusk"))),
            ["Lights off soon.", "Still awake?"]
        );
    }

    #[test]
    fn filters_by_voice_and_tag() {
        let (_p, chorus) = sample_chorus();
        assert_eq!(
            texts(&chorus, &filter(Some("drift"), Some("dusk"))),
            ["Lights off soon."]
        );
    }

    #[test]
    fn no_filter_matches_everything() {
        let (_p, chorus) = sample_chorus();
        assert_eq!(texts(&chorus, &Filter::default()).len(), 3);
    }

    #[test]
    fn no_match_returns_none() {
        let (_p, chorus) = sample_chorus();
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(
            chorus.pick(&filter(Some("ember"), Some("idle")), &mut rng),
            None
        );
        assert_eq!(chorus.pick(&filter(Some("ghost"), None), &mut rng), None);
        assert_eq!(chorus.pick(&filter(None, Some("nope")), &mut rng), None);
        assert_eq!(Chorus::default().pick(&Filter::default(), &mut rng), None);
    }

    #[test]
    fn same_seed_same_pick() {
        let (_p, chorus) = sample_chorus();
        let picks = |seed| {
            let mut rng = StdRng::seed_from_u64(seed);
            (0..20)
                .map(|_| chorus.pick(&Filter::default(), &mut rng).unwrap().text)
                .collect::<Vec<_>>()
        };
        assert_eq!(picks(42), picks(42));
    }

    #[test]
    fn interjection_carries_voice() {
        let (_p, chorus) = sample_chorus();
        let mut rng = StdRng::seed_from_u64(3);
        let i = chorus
            .pick(&filter(Some("drift"), Some("dusk")), &mut rng)
            .unwrap();
        assert_eq!(i.voice.key, "drift");
        assert_eq!(i.voice.display, "DRIFT");
        assert_eq!(i.voice.color.as_deref(), Some("#336699"));
    }

    #[test]
    fn respects_weights() {
        let packs = Packs::new();
        packs.add(
            "p",
            "[voices.drift]\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"rare\"\nweight = 1\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"common\"\nweight = 9\n",
        );
        let (chorus, _) = Chorus::load(packs.dir()).unwrap();
        let mut rng = StdRng::seed_from_u64(2026);

        let picks = 10_000;
        let common = (0..picks)
            .filter(|_| chorus.pick(&Filter::default(), &mut rng).unwrap().text == "common")
            .count();

        let share = common as f64 / picks as f64;
        assert!((0.85..=0.95).contains(&share), "common share was {share}");
    }

    /// Picks `n` times, feeding each pick into the history.
    fn run(
        chorus: &Chorus,
        filter: &Filter,
        seed: u64,
        n: usize,
    ) -> (Vec<Interjection>, Vec<Seen>) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut history = Vec::new();
        let mut picks = Vec::new();
        for _ in 0..n {
            let i = chorus
                .pick_with_history(filter, &history, &mut rng)
                .unwrap();
            history.push(Seen {
                id: i.id,
                voice: i.voice.key.clone(),
            });
            picks.push(i);
        }
        (picks, history)
    }

    fn one_voice(lines: usize) -> (Packs, Chorus) {
        let packs = Packs::new();
        let mut toml = "[voices.drift]\n".to_owned();
        for n in 0..lines {
            toml.push_str(&format!(
                "[[lines]]\nvoice = \"drift\"\ntext = \"line {n}\"\n"
            ));
        }
        packs.add("p", &toml);
        let (chorus, _) = Chorus::load(packs.dir()).unwrap();
        (packs, chorus)
    }

    fn two_voices(lines_each: usize) -> (Packs, Chorus) {
        let packs = Packs::new();
        let mut toml = "[voices.drift]\n[voices.ember]\n".to_owned();
        for n in 0..lines_each {
            for v in ["drift", "ember"] {
                toml.push_str(&format!("[[lines]]\nvoice = \"{v}\"\ntext = \"{v} {n}\"\n"));
            }
        }
        packs.add("p", &toml);
        let (chorus, _) = Chorus::load(packs.dir()).unwrap();
        (packs, chorus)
    }

    #[test]
    fn prefers_unseen_lines() {
        let (_p, chorus) = one_voice(6);
        for seed in 0..20 {
            let (picks, _) = run(&chorus, &Filter::default(), seed, 6);
            let mut texts: Vec<_> = picks.iter().map(|i| i.text.as_str()).collect();
            texts.sort();
            texts.dedup();
            assert_eq!(texts.len(), 6, "seed {seed}: {texts:?}");
        }
    }

    #[test]
    fn falls_back_to_oldest_half() {
        let (_p, chorus) = one_voice(6);
        // History: line 0 shown first (oldest) ... line 5 last.
        let history: Vec<Seen> = (0..6)
            .map(|n| Seen {
                id: crate::pack::line_id("drift", &format!("line {n}")),
                voice: "drift".into(),
            })
            .collect();
        let mut rng = StdRng::seed_from_u64(11);
        for _ in 0..200 {
            let i = chorus
                .pick_with_history(&Filter::default(), &history, &mut rng)
                .unwrap();
            assert!(
                ["line 0", "line 1", "line 2"].contains(&i.text.as_str()),
                "{} isn't in the older half",
                i.text
            );
        }
    }

    #[test]
    fn voices_take_turns() {
        let (_p, chorus) = two_voices(30);
        for seed in 0..10 {
            let (picks, _) = run(&chorus, &Filter::default(), seed, 50);
            for pair in picks.windows(2) {
                assert_ne!(pair[0].voice.key, pair[1].voice.key, "seed {seed}");
            }
        }
    }

    #[test]
    fn voice_turn_allows_only_voice() {
        let (_p, chorus) = one_voice(3);
        let (picks, _) = run(&chorus, &Filter::default(), 1, 10);
        assert_eq!(picks.len(), 10);
        assert!(picks.iter().all(|i| i.voice.key == "drift"));
    }

    #[test]
    fn voice_filter_skips_voice_turn() {
        let (_p, chorus) = two_voices(10);
        let filter = Filter {
            voice: Some("ember".into()),
            ..Filter::default()
        };
        let (picks, _) = run(&chorus, &filter, 3, 8);
        assert!(picks.iter().all(|i| i.voice.key == "ember"));
    }

    #[test]
    fn history_keeps_weights() {
        let packs = Packs::new();
        packs.add(
            "p",
            "[voices.drift]\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"rare\"\nweight = 1\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"common\"\nweight = 9\n",
        );
        let (chorus, _) = Chorus::load(packs.dir()).unwrap();
        // History about unrelated lines doesn't change the odds.
        let history = vec![Seen {
            id: 42,
            voice: "other".into(),
        }];
        let mut rng = StdRng::seed_from_u64(2026);
        let common = (0..10_000)
            .filter(|_| {
                chorus
                    .pick_with_history(&Filter::default(), &history, &mut rng)
                    .unwrap()
                    .text
                    == "common"
            })
            .count();
        let share = common as f64 / 10_000.0;
        assert!((0.85..=0.95).contains(&share), "common share was {share}");
    }

    /// One `late`, one `morning` and one untagged line, all weight 1.
    fn timed() -> (Packs, Chorus) {
        let packs = Packs::new();
        packs.add(
            "p",
            "[voices.drift]\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"night\"\ntags = [\"late\"]\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"dawn\"\ntags = [\"morning\", \"focus\"]\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"any\"\n",
        );
        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        (packs, chorus)
    }

    fn at(hours: u32, tag: Option<&str>) -> Filter {
        Filter {
            tag: tag.map(Into::into),
            context: Context::at(hours * 60),
            ..Filter::default()
        }
    }

    #[test]
    fn context_tags_restrict() {
        let (_p, chorus) = timed();
        assert_eq!(texts(&chorus, &at(12, None)), ["any"]);
        assert_eq!(texts(&chorus, &at(23, None)), ["any", "night"]);
        assert_eq!(texts(&chorus, &at(7, None)), ["any", "dawn"]);
        // A non-context tag doesn't lift the time restriction.
        assert!(texts(&chorus, &at(12, Some("focus"))).is_empty());
    }

    #[test]
    fn explicit_tag_overrides_context() {
        let (_p, chorus) = timed();
        assert_eq!(texts(&chorus, &at(12, Some("late"))), ["night"]);
        assert_eq!(texts(&chorus, &at(23, Some("morning"))), ["dawn"]);
    }

    #[test]
    fn no_context_means_no_context_lines() {
        let (_p, chorus) = timed();
        assert_eq!(texts(&chorus, &Filter::default()), ["any"]);
        let late = Filter {
            tag: Some("late".into()),
            ..Filter::default()
        };
        assert_eq!(texts(&chorus, &late), ["night"]);
    }

    #[test]
    fn custom_boost() {
        let (_p, chorus) = timed();
        let rules = crate::ContextRules {
            boost: 9.0,
            ..Default::default()
        };
        let filter = Filter {
            context: rules.at(23 * 60),
            ..Filter::default()
        };
        let mut rng = StdRng::seed_from_u64(8);
        let night = (0..10_000)
            .filter(|_| chorus.pick(&filter, &mut rng).unwrap().text == "night")
            .count();
        let share = night as f64 / 10_000.0;
        // 9 : 1 → 90%.
        assert!((0.86..=0.94).contains(&share), "night share was {share}");
    }

    #[test]
    fn fail_lines_need_explicit_tag() {
        let packs = Packs::new();
        packs.add(
            "p",
            "[voices.drift]\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"oops\"\ntags = [\"fail\"]\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"any\"\n",
        );
        let (chorus, _) = Chorus::load(packs.dir()).unwrap();
        for hour in [3, 7, 12, 23] {
            assert_eq!(texts(&chorus, &at(hour, None)), ["any"]);
            assert_eq!(texts(&chorus, &at(hour, Some("fail"))), ["oops"]);
        }
    }

    #[test]
    fn active_context_boosts() {
        let (_p, chorus) = timed();
        let mut rng = StdRng::seed_from_u64(8);
        let filter = at(23, None);
        let night = (0..10_000)
            .filter(|_| chorus.pick(&filter, &mut rng).unwrap().text == "night")
            .count();
        let share = night as f64 / 10_000.0;
        assert!((0.70..=0.80).contains(&share), "night share was {share}");
    }

    #[test]
    fn line_sound_overrides_voice_sound() {
        let packs = Packs::new();
        let dir = packs.dir().join("p");
        packs.add(
            "p",
            "[voices.drift]\nsound = \"voice.ogg\"\nicon = \"drift.png\"\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"own\"\nsound = \"line.ogg\"\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"default\"\n",
        );
        for f in ["voice.ogg", "line.ogg", "drift.png"] {
            std::fs::write(dir.join(f), b"").unwrap();
        }
        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");

        let mut rng = StdRng::seed_from_u64(5);
        let mut seen = 0;
        while seen != 0b11 {
            let i = chorus.pick(&Filter::default(), &mut rng).unwrap();
            assert_eq!(i.icon, Some(dir.join("drift.png")));
            match i.text.as_str() {
                "own" => {
                    assert_eq!(i.sound, Some(dir.join("line.ogg")));
                    seen |= 0b01;
                }
                _ => {
                    assert_eq!(i.sound, Some(dir.join("voice.ogg")));
                    seen |= 0b10;
                }
            }
        }
    }
}
