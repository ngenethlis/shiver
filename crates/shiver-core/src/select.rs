//! Picking a line to show.

use std::path::PathBuf;

use rand::Rng;
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;

use crate::library::Library;
use crate::pack::Voice;

/// Which lines may be picked. Empty fields match everything.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    /// Only lines from this voice key.
    pub voice: Option<String>,
    /// Only lines with this tag.
    pub tag: Option<String>,
}

/// A picked line, ready to show.
#[derive(Debug, Clone, PartialEq)]
pub struct Interjection {
    /// The voice saying the line.
    pub voice: Voice,
    /// The line's text.
    pub text: String,
    /// Sound to play: the line's own, else the voice's.
    pub sound: Option<PathBuf>,
    /// Icon to show: the voice's.
    pub icon: Option<PathBuf>,
}

impl Library {
    /// Picks a random line matching `filter`, weighted by each line's weight.
    ///
    /// Returns `None` if no line matches.
    pub fn pick<R: Rng + ?Sized>(&self, filter: &Filter, rng: &mut R) -> Option<Interjection> {
        let candidates: Vec<_> = self
            .lines
            .iter()
            .filter(|l| filter.voice.as_ref().is_none_or(|v| &l.voice == v))
            .filter(|l| filter.tag.as_ref().is_none_or(|t| l.tags.contains(t)))
            .collect();

        // Weights are validated at load time (finite, > 0), so this only fails
        // when there are no candidates.
        let index = WeightedIndex::new(candidates.iter().map(|l| l.weight)).ok()?;
        let line = candidates[index.sample(rng)];
        let voice = self.voices.get(&line.voice)?.clone();

        Some(Interjection {
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

    /// Two voices; `drift` has two lines (one tagged `late`), `ember` has one.
    fn library() -> (Packs, Library) {
        let packs = Packs::new();
        packs.add(
            "p",
            "[voices.drift]\ndisplay = \"DRIFT\"\ncolor = \"#336699\"\n\
             [voices.ember]\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"Lights off soon.\"\ntags = [\"late\"]\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"Look out the window.\"\ntags = [\"idle\"]\n\
             [[lines]]\nvoice = \"ember\"\ntext = \"Still awake?\"\ntags = [\"late\"]\n",
        );
        let (lib, warnings) = Library::load(packs.dir()).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        (packs, lib)
    }

    fn filter(voice: Option<&str>, tag: Option<&str>) -> Filter {
        Filter {
            voice: voice.map(Into::into),
            tag: tag.map(Into::into),
        }
    }

    /// Every text that `filter` can produce over many seeded picks.
    fn texts(lib: &Library, f: &Filter) -> Vec<String> {
        let mut rng = StdRng::seed_from_u64(7);
        let mut seen: Vec<String> = (0..200)
            .filter_map(|_| lib.pick(f, &mut rng))
            .map(|i| i.text)
            .collect();
        seen.sort();
        seen.dedup();
        seen
    }

    #[test]
    fn filters_by_voice() {
        let (_p, lib) = library();
        assert_eq!(
            texts(&lib, &filter(Some("drift"), None)),
            ["Lights off soon.", "Look out the window."]
        );
    }

    #[test]
    fn filters_by_tag() {
        let (_p, lib) = library();
        assert_eq!(
            texts(&lib, &filter(None, Some("late"))),
            ["Lights off soon.", "Still awake?"]
        );
    }

    #[test]
    fn filters_by_voice_and_tag() {
        let (_p, lib) = library();
        assert_eq!(
            texts(&lib, &filter(Some("drift"), Some("late"))),
            ["Lights off soon."]
        );
    }

    #[test]
    fn no_filter_matches_everything() {
        let (_p, lib) = library();
        assert_eq!(texts(&lib, &Filter::default()).len(), 3);
    }

    #[test]
    fn no_match_returns_none() {
        let (_p, lib) = library();
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(
            lib.pick(&filter(Some("ember"), Some("idle")), &mut rng),
            None
        );
        assert_eq!(lib.pick(&filter(Some("ghost"), None), &mut rng), None);
        assert_eq!(lib.pick(&filter(None, Some("nope")), &mut rng), None);
        assert_eq!(Library::default().pick(&Filter::default(), &mut rng), None);
    }

    #[test]
    fn same_seed_same_pick() {
        let (_p, lib) = library();
        let picks = |seed| {
            let mut rng = StdRng::seed_from_u64(seed);
            (0..20)
                .map(|_| lib.pick(&Filter::default(), &mut rng).unwrap().text)
                .collect::<Vec<_>>()
        };
        assert_eq!(picks(42), picks(42));
    }

    #[test]
    fn interjection_carries_voice() {
        let (_p, lib) = library();
        let mut rng = StdRng::seed_from_u64(3);
        let i = lib
            .pick(&filter(Some("drift"), Some("late")), &mut rng)
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
        let (lib, _) = Library::load(packs.dir()).unwrap();
        let mut rng = StdRng::seed_from_u64(2026);

        let picks = 10_000;
        let common = (0..picks)
            .filter(|_| lib.pick(&Filter::default(), &mut rng).unwrap().text == "common")
            .count();

        let share = common as f64 / picks as f64;
        assert!((0.85..=0.95).contains(&share), "common share was {share}");
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
        let (lib, warnings) = Library::load(packs.dir()).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");

        let mut rng = StdRng::seed_from_u64(5);
        let mut seen = 0;
        while seen != 0b11 {
            let i = lib.pick(&Filter::default(), &mut rng).unwrap();
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
