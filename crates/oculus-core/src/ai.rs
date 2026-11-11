//! CPU opponent, matching `lib/ai.js`.
use crate::rng::Rng;
use crate::{Base, BaseId, Team};

/// The CPU will not attack from a base holding fewer units than this.
const MIN_ATTACK_VALUE: f32 = 15.0;

fn team_total(bases: &[Base], team: Team) -> f32 {
    bases
        .iter()
        .filter(|b| b.team == team)
        .map(|b| b.value)
        .sum()
}

/// Picks the CPU's strongest base (first wins ties) and, if it is strong enough,
/// a random non-CPU target it can overpower. Returns `(from, to)`.
///
/// A target qualifies when `from.value * (cpu_units / player_units) > target.value`.
/// With no player units the advantage is overwhelming, so any target qualifies.
pub(crate) fn choose_attack(bases: &[Base], rng: &mut Rng) -> Option<(BaseId, BaseId)> {
    let from = bases
        .iter()
        .filter(|b| b.team == Team::Cpu)
        .fold(None::<&Base>, |best, b| match best {
            Some(best) if best.value >= b.value => Some(best),
            _ if b.value > 0.0 => Some(b),
            _ => best,
        })?;
    if from.value < MIN_ATTACK_VALUE {
        return None;
    }

    let cpu_units = team_total(bases, Team::Cpu);
    let player_units = team_total(bases, Team::Player);

    // Shuffle a copy of the candidates so game state keeps its order.
    let mut candidates: Vec<&Base> = bases.iter().filter(|b| b.team != Team::Cpu).collect();
    rng.shuffle(&mut candidates);
    candidates
        .into_iter()
        .find(|target| {
            player_units <= 0.0 || from.value * (cpu_units / player_units) > target.value
        })
        .map(|target| (from.id, target.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::make_base;
    use crate::Point;

    fn base(id: usize, team: Team, value: f32) -> Base {
        make_base(id, Point::new(100.0 * id as f32, 100.0), team, value)
    }

    #[test]
    fn never_attacks_below_threshold() {
        let bases = [
            base(0, Team::Player, 5.0),
            base(1, Team::Cpu, 14.9),
            base(2, Team::Neutral, 1.0),
        ];
        for seed in 0..100 {
            assert_eq!(choose_attack(&bases, &mut Rng::new(seed)), None);
        }
    }

    #[test]
    fn attacks_at_threshold_and_never_targets_cpu() {
        let bases = [
            base(0, Team::Player, 10.0),
            base(1, Team::Cpu, 15.0),
            base(2, Team::Cpu, 12.0),
            base(3, Team::Neutral, 10.0),
            base(4, Team::Neutral, 10.0),
        ];
        let mut seen_targets = [false; 5];
        for seed in 0..200 {
            let (from, to) = choose_attack(&bases, &mut Rng::new(seed)).expect("should attack");
            assert_eq!(from, 1);
            assert_ne!(bases[to].team, Team::Cpu);
            seen_targets[to] = true;
        }
        // The shuffle varies the target between seeds.
        assert!(seen_targets[3] && seen_targets[4]);
    }

    #[test]
    fn picks_the_largest_cpu_base() {
        let bases = [
            base(0, Team::Player, 5.0),
            base(1, Team::Cpu, 20.0),
            base(2, Team::Cpu, 40.0),
            base(3, Team::Neutral, 1.0),
        ];
        let (from, _) = choose_attack(&bases, &mut Rng::new(1)).unwrap();
        assert_eq!(from, 2);
    }

    #[test]
    fn only_targets_it_can_overpower() {
        // scale = 20/40 = 0.5, so strength is 10: only values below 10 qualify.
        let bases = [
            base(0, Team::Player, 40.0),
            base(1, Team::Cpu, 20.0),
            base(2, Team::Neutral, 30.0),
            base(3, Team::Neutral, 9.0),
        ];
        for seed in 0..50 {
            assert_eq!(choose_attack(&bases, &mut Rng::new(seed)), Some((1, 3)));
        }
        let strong = [base(0, Team::Player, 40.0), base(1, Team::Cpu, 20.0)];
        assert_eq!(choose_attack(&strong, &mut Rng::new(1)), None);
    }

    #[test]
    fn zero_player_units_is_finite_and_attacks_anything() {
        let bases = [
            base(0, Team::Cpu, 20.0),
            base(1, Team::Neutral, 40.0),
            base(2, Team::Neutral, 40.0),
        ];
        for seed in 0..50 {
            let (from, to) = choose_attack(&bases, &mut Rng::new(seed)).unwrap();
            assert_eq!(from, 0);
            assert!(to == 1 || to == 2);
        }
    }

    #[test]
    fn no_cpu_bases_or_no_targets_means_no_attack() {
        let no_cpu = [base(0, Team::Player, 30.0), base(1, Team::Neutral, 10.0)];
        assert_eq!(choose_attack(&no_cpu, &mut Rng::new(1)), None);
        let no_targets = [base(0, Team::Cpu, 30.0)];
        assert_eq!(choose_attack(&no_targets, &mut Rng::new(1)), None);
        assert_eq!(choose_attack(&[], &mut Rng::new(1)), None);
    }

    #[test]
    fn does_not_reorder_the_input() {
        let bases = [
            base(0, Team::Player, 10.0),
            base(1, Team::Cpu, 30.0),
            base(2, Team::Neutral, 10.0),
        ];
        let before = bases.clone();
        choose_attack(&bases, &mut Rng::new(9));
        assert_eq!(bases, before);
    }
}
