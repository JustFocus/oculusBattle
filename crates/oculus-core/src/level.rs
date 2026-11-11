//! Level/base generation, matching `Game.prototype.generateBases` and `addBases`.
use crate::rng::Rng;
use crate::{Base, Point, Team, WORLD_HEIGHT};

/// Start values neutral bases are drawn from; repeats weight the odds.
pub(crate) const BASE_SIZES: [u32; 48] = [
    10, 10, 10, 10, 10, 10, 10, 10, 10, //
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, //
    20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, //
    25, 25, 25, 25, 25, 25, //
    30, 30, 30, 30, //
    35, 35, //
    40,
];

pub(crate) const HOME_VALUE: f32 = 30.0;
pub(crate) const PLAYER_HOME: Point = Point::new(75.0, 250.0);
pub(crate) const CPU_HOME: Point = Point::new(725.0, 250.0);

const PADDING: f32 = 10.0;
const MAX_SIZE: f32 = 50.0;
/// Radius of a base is its value times this.
const RADIUS_PER_VALUE: f32 = 1.5;

pub(crate) fn make_base(id: usize, pos: Point, team: Team, value: f32) -> Base {
    Base {
        id,
        pos,
        radius: value * RADIUS_PER_VALUE,
        team,
        value,
        start_value: value,
    }
}

/// Generates the two home bases plus two columns of 1-3 neutral bases each.
/// The first column is mirrored at +350 so the map is symmetric in x.
/// Base ids equal their index in the returned vector.
pub(crate) fn generate_bases(rng: &mut Rng) -> Vec<Base> {
    let mut bases = vec![
        make_base(0, PLAYER_HOME, Team::Player, HOME_VALUE),
        make_base(1, CPU_HOME, Team::Cpu, HOME_VALUE),
    ];
    for column in 0..2 {
        let count = rng.below(3) + 1;
        let vertical_split = (WORLD_HEIGHT / count as f32).floor();
        let x = 50.0 + (column + 1) as f32 * 175.0;
        for slot in 0..count {
            let size = BASE_SIZES[rng.below(BASE_SIZES.len())] as f32;
            let min_y = slot as f32 * vertical_split + PADDING + MAX_SIZE;
            let max_y = (slot + 1) as f32 * vertical_split - PADDING - MAX_SIZE;
            let y = (rng.next_f32() * (max_y - min_y + 1.0) + min_y).floor();
            bases.push(make_base(
                bases.len(),
                Point::new(x, y),
                Team::Neutral,
                size,
            ));
            if column == 0 {
                bases.push(make_base(
                    bases.len(),
                    Point::new(x + 350.0, y),
                    Team::Neutral,
                    size,
                ));
            }
        }
    }
    bases
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WORLD_WIDTH;

    #[test]
    fn invariants_hold_for_many_seeds() {
        for seed in 0..500 {
            let bases = generate_bases(&mut Rng::new(seed));
            // 2 homes + column 1 (1-3, mirrored: 2-6) + column 2 (1-3).
            assert!((5..=11).contains(&bases.len()), "len {}", bases.len());
            assert_eq!(bases[0].team, Team::Player);
            assert_eq!(bases[0].pos, PLAYER_HOME);
            assert_eq!(bases[1].team, Team::Cpu);
            assert_eq!(bases[1].pos, CPU_HOME);
            for (i, b) in bases.iter().enumerate() {
                assert_eq!(b.id, i);
                assert_eq!(b.radius, b.value * 1.5);
                assert_eq!(b.value, b.start_value);
                assert!(b.pos.x - b.radius >= 0.0 && b.pos.x + b.radius <= WORLD_WIDTH);
                assert!(b.pos.y - b.radius >= 0.0 && b.pos.y + b.radius <= WORLD_HEIGHT);
                if i >= 2 {
                    assert_eq!(b.team, Team::Neutral);
                    assert!(BASE_SIZES.contains(&(b.value as u32)));
                }
            }
        }
    }

    #[test]
    fn first_column_is_mirrored() {
        let bases = generate_bases(&mut Rng::new(11));
        // Neutrals start at index 2 and the first column comes in (left, right) pairs.
        assert_eq!(bases[2].pos.x, 225.0);
        assert_eq!(bases[3].pos.x, 575.0);
        assert_eq!(bases[2].pos.y, bases[3].pos.y);
        assert_eq!(bases[2].value, bases[3].value);
    }

    #[test]
    fn generation_is_deterministic() {
        assert_eq!(
            generate_bases(&mut Rng::new(5)),
            generate_bases(&mut Rng::new(5))
        );
    }
}
