//! Actor movement and ghost AI (private engine half of `sim`).
//!
//! Movement model: positions are whole pixels (Fix8 raw values that are
//! multiples of 256); each actor carries a Fix8 per-tick budget accumulator
//! (`accum`). A tick adds `speed` to the accumulator and performs one
//! 1-pixel logical step per 256 accumulated, so tile-entry decisions and
//! center turns happen at exact pixel positions like the arcade
//! (dossier §1.4, §2.1).

use super::{Game, step_tile, timings};
use crate::types::{Dir, Fix8, GhostId, GhostState, InputFrame, Mode, PxPos, TilePos};

/// Frightened turn try-order: the ROM tries the PRNG direction first, then
/// proceeds clockwise (dossier §3.8: up → right → down → left).
const CLOCKWISE: [Dir; 4] = [Dir::Up, Dir::Right, Dir::Down, Dir::Left];

impl Game {
    // --- Pac-Man ---------------------------------------------------------

    /// Phase 2: Pac-Man movement — eating-pause, buffered turns, cornering,
    /// wall stop, warp (dossier §2.2–§2.5).
    pub(super) fn move_pac(&mut self, input: InputFrame) {
        if self.pac.pause > 0 {
            // Eating pause: 1 tick per dot, 3 per energizer (dossier §2.4).
            self.pac.pause -= 1;
            return;
        }
        self.pac.accum += self.pac_speed();
        while self.pac.accum >= 256 {
            self.pac.accum -= 256;
            if !self.pac_step(input.dir) {
                // Blocked by a wall: drop the remaining budget so speed does
                // not accumulate against the wall.
                self.pac.accum = 0;
                break;
            }
        }
    }

    /// One 1-pixel Pac-Man step. Returns false when blocked by a wall.
    fn pac_step(&mut self, want: Option<Dir>) -> bool {
        // Finish an in-progress corner: 1 px in the new direction plus 1 px
        // toward the new path's centerline per step — the 45° cut at
        // effectively double speed (dossier §2.2).
        if self.pac.corner_left > 0 {
            let (dx, dy) = self.pac.dir.delta();
            let (cx, cy) = self.pac.corner_step;
            let p = &mut self.pac;
            p.pos.x = Fix8(p.pos.x.raw() + (dx + cx) * 256);
            p.pos.y = Fix8(p.pos.y.raw() + (dy + cy) * 256);
            p.corner_left -= 1;
            if p.corner_left == 0 {
                p.corner_step = (0, 0);
            }
            p.anim_px += 1;
            self.pac.pos = self.map.warp(self.pac.pos);
            return true;
        }
        if let Some(w) = want {
            if w == self.pac.dir.opposite() {
                // Reversal is allowed at any time (dossier §2.3).
                self.pac.dir = w;
            } else if w != self.pac.dir && self.try_turn(w) {
                // Turn accepted (possibly starting a corner); take the step.
                return self.pac_step(None);
            }
        }
        // Straight step, stopping at the tile center before a wall.
        let t = self.pac.pos.tile();
        let center = PxPos::tile_center(t);
        let at_center = match self.pac.dir {
            Dir::Left | Dir::Right => self.pac.pos.x == center.x,
            Dir::Up | Dir::Down => self.pac.pos.y == center.y,
        };
        if at_center
            && !self
                .map
                .walkable(self.wrap_tile(step_tile(t, self.pac.dir)))
        {
            return false;
        }
        let (dx, dy) = self.pac.dir.delta();
        let p = &mut self.pac;
        p.pos.x = Fix8(p.pos.x.raw() + dx * 256);
        p.pos.y = Fix8(p.pos.y.raw() + dy * 256);
        p.anim_px += 1;
        self.pac.pos = self.map.warp(self.pac.pos);
        true
    }

    /// Attempt a buffered perpendicular turn at the current tile. The
    /// held direction is continuously tested and takes effect at the first
    /// legal pixel (dossier §2.3); anywhere inside the turn tile counts,
    /// giving the pre/post-turn window of §2.2 (see module docs for the
    /// one-pixel center-convention shift).
    fn try_turn(&mut self, w: Dir) -> bool {
        let t = self.pac.pos.tile();
        if !self.map.walkable(self.wrap_tile(step_tile(t, w))) {
            return false;
        }
        let center = PxPos::tile_center(t);
        // Pixels to the new path's centerline along the current travel axis.
        let diff_px = match w {
            Dir::Up | Dir::Down => (center.x.raw() - self.pac.pos.x.raw()) / 256,
            Dir::Left | Dir::Right => (center.y.raw() - self.pac.pos.y.raw()) / 256,
        };
        self.pac.dir = w;
        if diff_px != 0 {
            self.pac.corner_left = diff_px.abs();
            self.pac.corner_step = match w {
                Dir::Up | Dir::Down => (diff_px.signum(), 0),
                Dir::Left | Dir::Right => (0, diff_px.signum()),
            };
        }
        true
    }

    // --- ghost dispatch ---------------------------------------------------

    /// Phase 6: move every ghost according to its state, in fixed
    /// Blinky/Pinky/Inky/Clyde order (determinism).
    pub(super) fn move_ghosts(&mut self) {
        for i in 0..4 {
            match self.ghosts[i].state {
                GhostState::InHouse => self.house_bounce(i),
                GhostState::Leaving => self.house_leave(i),
                GhostState::Entering => self.house_enter(i),
                GhostState::Eyes => self.move_eyes(i),
                GhostState::Active => self.ghost_move(i),
            }
        }
    }

    fn ghost_move(&mut self, i: usize) {
        self.ghosts[i].accum += self.ghost_speed(i);
        while self.ghosts[i].accum >= 256 {
            self.ghosts[i].accum -= 256;
            self.ghost_step(i);
        }
    }

    /// Eyes on the board: exactly 2 px/tick (supplements §2), normal
    /// decision logic toward `Map::eyes_target`, arrival at the exact
    /// outside-door point flips to Entering (supplements §8).
    pub(super) fn move_eyes(&mut self, i: usize) {
        self.ghosts[i].accum += self.ghost_speed(i);
        while self.ghosts[i].accum >= 256 {
            self.ghosts[i].accum -= 256;
            self.ghost_step(i);
            if self.ghosts[i].pos == self.house_exit_point() {
                self.ghosts[i].state = GhostState::Entering;
                self.ghosts[i].accum = 0;
                break;
            }
        }
    }

    /// One 1-pixel ghost step: apply the stored turn at the tile center,
    /// move, warp, and run the look-ahead decision on tile entry
    /// (dossier §3.4).
    fn ghost_step(&mut self, i: usize) {
        let t = self.ghosts[i].pos.tile();
        if self.ghosts[i].pos == PxPos::tile_center(t) {
            self.ghosts[i].dir = self.ghosts[i].next_dir;
        }
        let (dx, dy) = self.ghosts[i].dir.delta();
        let g = &mut self.ghosts[i];
        g.pos.x = Fix8(g.pos.x.raw() + dx * 256);
        g.pos.y = Fix8(g.pos.y.raw() + dy * 256);
        g.pos = self.map.warp(g.pos);
        let nt = self.ghosts[i].pos.tile();
        if nt != t {
            self.ghost_enter_tile(i, nt);
        }
    }

    /// Tile-entry bookkeeping: consume a pending forced reversal (dossier
    /// §3.3: obeyed "when it next enters a new tile"), otherwise promote the
    /// stored decision and compute the next one for the tile one ahead.
    fn ghost_enter_tile(&mut self, i: usize, nt: TilePos) {
        let nd = if self.ghosts[i].reverse_pending {
            self.ghosts[i].reverse_pending = false;
            let nd = self.ghosts[i].dir.opposite();
            self.ghosts[i].dir = nd;
            nd
        } else {
            self.ghosts[i].future_dir
        };
        self.ghosts[i].next_dir = nd;
        let ahead = self.wrap_tile(step_tile(nt, nd));
        self.ghosts[i].future_dir = self.choose_dir(i, ahead, nd);
    }

    /// Decide the exit a ghost will take from look-ahead tile `t2`, entered
    /// travelling `travel` (dossier §3.4): discard walls and the reverse,
    /// then minimize Euclidean distance (squared) from the test tile to the
    /// target, tie-breaking Up > Left > Down > Right. Red zones ban Up for
    /// scatter/chase ghosts only (dossier §3.7; frightened and eyes exempt,
    /// supplements §12). Frightened ghosts pick randomly instead
    /// (dossier §3.8). A dead end (no legal exit) falls back to reversing —
    /// unreachable on the classic maze.
    pub(super) fn choose_dir(&mut self, i: usize, t2: TilePos, travel: Dir) -> Dir {
        let g = &self.ghosts[i];
        if g.state == GhostState::Active && g.frightened {
            return self.choose_frightened(t2, travel);
        }
        let target = self.target_tile(i);
        let banned_up =
            g.state == GhostState::Active && !g.frightened && self.map.no_up_tiles().contains(&t2);
        let mut best: Option<(i64, Dir)> = None;
        for cand in Dir::IN_PRIORITY_ORDER {
            if cand == travel.opposite() {
                continue;
            }
            if cand == Dir::Up && banned_up {
                continue;
            }
            let test = step_tile(t2, cand);
            if !self.map.walkable(self.wrap_tile(test)) {
                continue;
            }
            let dist = test.dist_sq(target);
            let better = match best {
                None => true,
                Some((bd, _)) => dist < bd, // strict: earlier direction wins ties
            };
            if better {
                best = Some((dist, cand));
            }
        }
        best.map_or(travel.opposite(), |(_, d)| d)
    }

    /// Frightened random turn: PRNG picks the first try, then clockwise
    /// retries until one is not a wall and not the reverse (dossier §3.8).
    /// RNG consumption point 1 (see module docs).
    fn choose_frightened(&mut self, t2: TilePos, travel: Dir) -> Dir {
        let start = (self.rng.next_u32() & 3) as usize;
        for k in 0..4 {
            let cand = CLOCKWISE[(start + k) % 4];
            if cand == travel.opposite() {
                continue;
            }
            if self.map.walkable(self.wrap_tile(step_tile(t2, cand))) {
                return cand;
            }
        }
        travel.opposite()
    }

    /// Current pathfinding target for ghost `i` (must not be called for
    /// frightened deciders): eyes home tile, scatter corner, or chase target
    /// — with the Cruise-Elroy scatter override for Blinky (dossier §3.11).
    pub(super) fn target_tile(&self, i: usize) -> TilePos {
        if self.ghosts[i].state == GhostState::Eyes {
            return self.map.eyes_target();
        }
        let id = GhostId::ALL[i];
        let scatter = self.map.scatter_target(id);
        let elroy = i == 0 && self.elroy_stage() > 0;
        if self.mode == Mode::Scatter && !elroy {
            return scatter; // dossier §3.5
        }
        chase_target(
            id,
            self.pac.pos.tile(),
            self.pac.dir,
            self.ghosts[0].pos.tile(),
            self.ghosts[i].pos.tile(),
            scatter,
        )
    }

    // --- ghost house ------------------------------------------------------

    /// The point just outside the door where exits complete and eyes arrive:
    /// house-center x on the eyes-target row's centerline (supplements §8:
    /// arcade pixel (0x80, 0x64); classic (112, 116)).
    pub(super) fn house_exit_point(&self) -> PxPos {
        PxPos {
            x: self.map.house_center().x,
            y: Fix8::from_px(
                self.map.eyes_target().y * crate::types::TILE_PX + crate::types::TILE_PX / 2,
            ),
        }
    }

    /// Revival slot x: the ghost's own spawn x for in-house spawners
    /// (Inky/Clyde shift sideways), else the house center (supplements §8).
    fn slot_x(&self, i: usize) -> i32 {
        let id = GhostId::ALL[i];
        let spawn = self.map.ghost_spawn(id);
        if self.map.cell(spawn.tile()) == crate::map::Cell::House {
            spawn.x.raw()
        } else {
            self.map.house_center().x.raw()
        }
    }

    /// Housed ghosts bounce vertically ±4 px around their slot at 0.5
    /// px/tick (supplements §10). Also runs during the death freeze.
    pub(super) fn house_bounce(&mut self, i: usize) {
        self.ghosts[i].accum += self.ghost_speed(i);
        let spawn_y = self.map.ghost_spawn(GhostId::ALL[i]).y.raw();
        let low = spawn_y - timings::HOUSE_BOUNCE_PX * 256;
        let high = spawn_y + timings::HOUSE_BOUNCE_PX * 256;
        while self.ghosts[i].accum >= 256 {
            self.ghosts[i].accum -= 256;
            let g = &mut self.ghosts[i];
            if !matches!(g.dir, Dir::Up | Dir::Down) {
                g.dir = Dir::Up;
            }
            let dy = if g.dir == Dir::Up { -256 } else { 256 };
            let mut ny = g.pos.y.raw() + dy;
            if ny < low || ny > high {
                g.dir = g.dir.opposite();
                ny = g.pos.y.raw() - dy;
            }
            g.pos.y = Fix8(ny);
        }
    }

    /// Leaving the house at 0.5 px/tick: align x to the house-center
    /// column, rise to the outside-door point, then start play facing left
    /// (right after a mode change while housed) — supplements §10,
    /// dossier §3.9.
    pub(super) fn house_leave(&mut self, i: usize) {
        self.ghosts[i].accum += self.ghost_speed(i);
        let exit = self.house_exit_point();
        while self.ghosts[i].accum >= 256 {
            self.ghosts[i].accum -= 256;
            let g = &mut self.ghosts[i];
            let (px, py) = (g.pos.x.raw(), g.pos.y.raw());
            if px != exit.x.raw() {
                let s = (exit.x.raw() - px).signum();
                g.dir = if s > 0 { Dir::Right } else { Dir::Left };
                g.pos.x = Fix8(px + s * 256);
            } else if py != exit.y.raw() {
                let s = (exit.y.raw() - py).signum();
                g.dir = if s > 0 { Dir::Down } else { Dir::Up };
                g.pos.y = Fix8(py + s * 256);
            } else {
                self.activate_ghost(i);
                break;
            }
        }
        // Zero-distance exit (degenerate maps): activate without a step.
        if self.ghosts[i].state == GhostState::Leaving && self.ghosts[i].pos == exit {
            self.activate_ghost(i);
        }
    }

    /// Transition Leaving -> Active at the outside-door point: face LEFT
    /// (or RIGHT if a mode change happened while housed) and seed the
    /// look-ahead decision (supplements §7/§10, dossier §3.9).
    fn activate_ghost(&mut self, i: usize) {
        let dir = if self.ghosts[i].exit_right {
            Dir::Right
        } else {
            Dir::Left
        };
        let g = &mut self.ghosts[i];
        g.state = GhostState::Active;
        g.dir = dir;
        g.next_dir = dir;
        g.exit_right = false;
        g.accum = 0;
        let t = self.ghosts[i].pos.tile();
        let ahead = self.wrap_tile(step_tile(t, dir));
        self.ghosts[i].future_dir = self.choose_dir(i, ahead, dir);
    }

    /// Eyes descending into the house at 2 px/tick: down to the house-center
    /// row, sideways to the revival slot, then revive to a housed ghost
    /// (supplements §8). Runs during the ghost-eaten pause too.
    pub(super) fn house_enter(&mut self, i: usize) {
        self.ghosts[i].accum += self.ghost_speed(i);
        let hc_y = self.map.house_center().y.raw();
        let slot_x = self.slot_x(i);
        while self.ghosts[i].accum >= 256 {
            self.ghosts[i].accum -= 256;
            let g = &mut self.ghosts[i];
            let (px, py) = (g.pos.x.raw(), g.pos.y.raw());
            if py != hc_y {
                let s = (hc_y - py).signum();
                g.dir = if s > 0 { Dir::Down } else { Dir::Up };
                g.pos.y = Fix8(py + s * 256);
            } else if px != slot_x {
                let s = (slot_x - px).signum();
                g.dir = if s > 0 { Dir::Right } else { Dir::Left };
                g.pos.x = Fix8(px + s * 256);
            } else {
                g.state = GhostState::InHouse;
                g.dir = Dir::Up;
                g.frightened = false;
                g.accum = 0;
                break;
            }
        }
        // Zero-distance revive (degenerate maps).
        let g = &mut self.ghosts[i];
        if g.state == GhostState::Entering && g.pos.y.raw() == hc_y && g.pos.x.raw() == slot_x {
            g.state = GhostState::InHouse;
            g.dir = Dir::Up;
            g.frightened = false;
            g.accum = 0;
        }
    }
}

/// Pure arcade chase-targeting formula (dossier §3.6), exposed for direct
/// verification against the dossier's worked examples:
///
/// - Blinky: Pac-Man's tile.
/// - Pinky: 4 tiles ahead of Pac-Man — facing Up, ALSO 4 left (overflow bug).
/// - Inky: pivot 2 ahead of Pac-Man (same Up bug), then double the vector
///   from Blinky's tile to the pivot (`2*pivot − blinky`).
/// - Clyde: Pac-Man's tile at ≥ 8 tiles Euclidean distance, else his own
///   scatter corner.
pub fn chase_target(
    ghost: GhostId,
    pac_tile: TilePos,
    pac_dir: Dir,
    blinky_tile: TilePos,
    own_tile: TilePos,
    scatter_target: TilePos,
) -> TilePos {
    match ghost {
        GhostId::Blinky => pac_tile,
        GhostId::Pinky => ahead_with_overflow(pac_tile, pac_dir, 4),
        GhostId::Inky => {
            let p = ahead_with_overflow(pac_tile, pac_dir, 2);
            TilePos::new(2 * p.x - blinky_tile.x, 2 * p.y - blinky_tile.y)
        }
        GhostId::Clyde => {
            if own_tile.dist_sq(pac_tile) >= 64 {
                pac_tile
            } else {
                scatter_target
            }
        }
    }
}

/// `n` tiles ahead of `t` facing `d`, reproducing the arcade's Up-overflow
/// bug: facing Up also shifts `n` tiles left (dossier §3.6).
fn ahead_with_overflow(t: TilePos, d: Dir, n: i32) -> TilePos {
    let (dx, dy) = d.delta();
    let mut r = TilePos::new(t.x + dx * n, t.y + dy * n);
    if d == Dir::Up {
        r.x -= n;
    }
    r
}
