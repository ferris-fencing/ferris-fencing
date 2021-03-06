pub const GAME_FIELD_SIZE: i32 = 10;
pub const P1_START_POS: i32 = 1;
pub const P2_START_POS: i32 = 8;
pub const START_ENERGY: i32 = 30000;
pub const GAMES_PER_MATCH: usize = 1;
pub const MAX_TURNS: i32 = 20;

#[derive(Serialize, Deserialize)]
pub struct Match {
    pub games: Vec<Game>,
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct Game {
    pub turns: Vec<Turn>,
    pub end: EndState,
}

#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, Debug)]
pub struct Turn {
    pub state: ActiveState,
    pub moves: MovePair,
}

#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, Debug)]
pub struct ActiveState {
    pub p1: PlayerState,
    pub p2: PlayerState,
}

#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, Debug)]
pub struct PlayerState {
    pub pos: i32,
    pub energy: i32,
}

#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, Debug)]
pub enum EndState {
    P1Victory(ActiveState),
    P2Victory(ActiveState),
    P1Pin(ActiveState),
    P2Pin(ActiveState),
    P1Survive(ActiveState),
    P2Survive(ActiveState),
    P1Energy(ActiveState),
    P2Energy(ActiveState),
    EnergyTie(ActiveState),
    P1Turns(ActiveState),
    P2Turns(ActiveState),
    TurnTie(ActiveState),
}

#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, Debug)]
pub struct MovePair {
    pub p1: Move,
    pub p2: Move,
}

#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, Debug)]
pub struct Move {
    pub kind: MoveKind,
    pub energy_spent: i32,
}

#[derive(Serialize, Deserialize)]
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum MoveKind {
    Back,
    Stand,
    Forward,
    Lunge,
    NoEnergy,
}

#[derive(Debug)]
pub enum NextGameState {
    Active(ActiveState),
    End(EndState),
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Player { P1, P2 }

#[derive(Debug)]
pub struct DecisionState {
    p1_dist_from_wall: i32,
    p2_dist_from_wall: i32,
    separation_dist: i32,
    p1_energy: i32,
    p2_energy: i32,
}

impl ActiveState {
    pub fn assert(&self) {
        assert!(self.p1.pos >= 0);
        assert!(self.p2.pos >= 0);
        assert!(self.p1.pos < GAME_FIELD_SIZE);
        assert!(self.p2.pos < GAME_FIELD_SIZE);
        assert!(self.p1.pos != self.p2.pos);
        assert!(self.p1.pos < self.p2.pos);

        assert!(self.p1.energy > 0);
        assert!(self.p2.energy > 0);
        assert!(self.p1.energy <= START_ENERGY);
        assert!(self.p2.energy <= START_ENERGY);
    }

    pub fn decision_state(&self) -> DecisionState {
        let ds = DecisionState {
            p1_dist_from_wall: self.p1.pos,
            p2_dist_from_wall: (GAME_FIELD_SIZE - 1) - self.p2.pos,
            separation_dist: self.p2.pos - self.p1.pos - 1,
            p1_energy: self.p1.energy,
            p2_energy: self.p2.energy,
        };

        debug!("as: {:?}", self);
        debug!("ds: {:?}", ds);

        ds
    }

    pub fn make_move(self, moves: MovePair, turn_no: i32) -> (Turn, NextGameState) {
        use crate::transition::{self, WallOrientation, Separation, Transition};
        use WallOrientation::*;
        use MoveKind::*;
        use NextGameState::*;
        use Separation::*;
        use std::cmp::Ordering::*;
        use crate::transition::Transition::*;

        self.assert();

        let ds = self.decision_state();

        assert!(ds.p1_energy > moves.p1.energy_spent || moves.p1.kind == MoveKind::NoEnergy);
        assert!(ds.p2_energy > moves.p2.energy_spent || moves.p2.kind == MoveKind::NoEnergy);

        let sep = match ds.separation_dist {
            0 => S0, 1 => S1, 2 => S2, 3 => S3, _ => SG,
        };
        let p1_move = moves.p1.kind;
        let p2_move = moves.p2.kind;
        let p1_wall = if ds.p1_dist_from_wall > 0 { NotAgainst } else { Against };
        let p2_wall = if ds.p2_dist_from_wall > 0 { NotAgainst } else { Against };

        let turn = Turn {
            state: self,
            moves
        };

        if turn_no == MAX_TURNS {
            let ngs = match self.p1.energy.cmp(&self.p2.energy) {
                Less => EndState::P2Turns(self),
                Equal => EndState::TurnTie(self),
                Greater => EndState::P1Turns(self),
            };
            return (turn, End(ngs));
        }

        let nm = self.naive_moves(moves);
        let bouncem = ActiveState {
            p1: PlayerState { pos: self.p1.pos, energy: nm.p1.energy, },
            p2: PlayerState { pos: self.p2.pos, energy: nm.p2.energy, },
        };
        let clampedm = ActiveState {
            p1: PlayerState {
                pos: if nm.p1.pos < 0 { 0 } else if nm.p1.pos >= GAME_FIELD_SIZE { GAME_FIELD_SIZE - 1 } else { nm.p1.pos },
                energy: nm.p1.energy,
            },
            p2: PlayerState {
                pos: if nm.p2.pos < 0 { 0 } else if nm.p2.pos >= GAME_FIELD_SIZE { GAME_FIELD_SIZE - 1 } else { nm.p2.pos },
                energy: nm.p2.energy,
            },
        };
        let energym = ActiveState {
            p1: PlayerState { pos: self.p1.pos, energy: nm.p1.energy },
            p2: PlayerState { pos: self.p2.pos, energy: nm.p2.energy },
        };

        let anm = Active(nm);
        let bounce = Active(bouncem);
        let wall = Active(clampedm);
        let p1v = End(EndState::P1Victory(nm));
        let p2v = End(EndState::P2Victory(nm));
        let p1survive = End(EndState::P1Survive(energym));
        let p2survive = End(EndState::P2Survive(energym));
        let p1push = Active(ActiveState {
            p1: PlayerState { pos: self.p2.pos, energy: nm.p1.energy },
            p2: PlayerState { pos: self.p2.pos.wrapping_add(1), energy: nm.p2.energy, },
        });
        let p2push = Active(ActiveState {
            p1: PlayerState { pos: self.p1.pos.wrapping_sub(1), energy: nm.p1.energy, },
            p2: PlayerState { pos: self.p1.pos, energy: nm.p2.energy, },
        });
        let energy = End(match nm.p1.energy.cmp(&nm.p2.energy) {
            Less => EndState::P2Energy(energym),
            Equal => EndState::EnergyTie(energym),
            Greater => EndState::P1Energy(energym),
        });
        let p1pin = End(EndState::P1Pin(ActiveState {
            p1: PlayerState { pos: GAME_FIELD_SIZE - 1, energy: nm.p1.energy },
            p2: PlayerState { pos: GAME_FIELD_SIZE - 1, energy: nm.p2.energy },
        }));
        let p2pin = End(EndState::P1Pin(ActiveState {
            p1: PlayerState { pos: 0, energy: nm.p1.energy },
            p2: PlayerState { pos: 0, energy: nm.p2.energy },
        }));

        let trans = transition::go(sep, p1_move, p2_move, p1_wall, p2_wall);

        let ngs = match trans {
            ActiveNaiveMove => anm,
            ActiveBounce => bounce,
            ActiveP1Push => p1push,
            ActiveP2Push => p2push,
            ActiveWall => wall,
            EndP1Victory => p1v,
            EndP2Victory => p2v,
            EndP1Pin => p1pin,
            EndP2Pin => p2pin,
            EndP1Survive => p1survive,
            EndP2Survive => p2survive,
            EndEnergy => energy,
        };

        debug!("ngs: {:?}", ngs);

        ngs.assert();

        (turn, ngs)
    }

    fn naive_moves(&self, moves: MovePair) -> ActiveState {
        let next = ActiveState {
            p1: PlayerState {
                pos: self.naive_adj_p1(moves.p1.kind),
                energy: self.p1.energy.checked_sub(moves.p1.energy_spent).expect("naive p1 energy"),
            },
            p2: PlayerState {
                pos: self.naive_adj_p2(moves.p2.kind),
                energy: self.p2.energy.checked_sub(moves.p2.energy_spent).expect("naive p2 energy"),
            },
        };

        next
    }

    fn naive_adj_p1(&self, kind: MoveKind) -> i32 {
        use MoveKind::*;

        match kind {
            Back => self.p1.pos.checked_sub(1).expect("naive_adj_p1 back"),
            Stand => self.p1.pos,
            Forward => self.p1.pos.checked_add(1).expect("naive_adj_p1 forward"),
            Lunge => self.p1.pos.checked_add(2).expect("naive_adj_p1 lunge"),
            NoEnergy => self.p1.pos,
        }
    }

    fn naive_adj_p2(&self, kind: MoveKind) -> i32 {
        use MoveKind::*;

        match kind {
            Back => self.p2.pos.checked_add(1).expect("naive_adj_p2 back"),
            Stand => self.p2.pos,
            Forward => self.p2.pos.checked_sub(1).expect("naive_adj_p2 forward"),
            Lunge => self.p2.pos.checked_sub(2).expect("naive_adj_p2 lunge"),
            NoEnergy => self.p2.pos,
        }
    }
}

impl EndState {
    pub fn assert(&self) {
        use EndState::*;

        let s = match self {
            P1Victory(ref s) => s,
            P2Victory(ref s) => s,
            P1Pin(ref s) => s,
            P2Pin(ref s) => s,
            P1Survive(ref s) => s,
            P2Survive(ref s) => s,
            P1Energy(ref s) => s,
            P2Energy(ref s) => s,
            EnergyTie(ref s) => s,
            P1Turns(ref s) => s,
            P2Turns(ref s) => s,
            TurnTie(ref s) => s,
        };

        assert!(s.p1.pos >= 0);
        assert!(s.p2.pos >= 0);
        assert!(s.p1.pos < GAME_FIELD_SIZE);
        assert!(s.p2.pos < GAME_FIELD_SIZE);
        //assert!(s.p1.pos != s.p2.pos);
        //assert!(s.p1.pos < s.p2.pos);

        assert!(s.p1.energy <= START_ENERGY);
        assert!(s.p2.energy <= START_ENERGY);
    }

    pub fn inner_state(&self) -> ActiveState {
        use EndState::*;

        match *self {
            P1Victory(s) => s,
            P2Victory(s) => s,
            P1Pin(s) => s,
            P2Pin(s) => s,
            P1Survive(s) => s,
            P2Survive(s) => s,
            P1Energy(s) => s,
            P2Energy(s) => s,
            EnergyTie(s) => s,
            P1Turns(s) => s,
            P2Turns(s) => s,
            TurnTie(s) => s,
        }
    }

    pub fn winner(&self) -> &'static str {
        use EndState::*;

        match *self {
            P1Victory(s) => "player-1",
            P2Victory(s) => "player-2",
            P1Pin(s) => "player-1",
            P2Pin(s) => "player-2",
            P1Survive(s) => "player-1",
            P2Survive(s) => "player-2",
            P1Energy(s) => "player-1",
            P2Energy(s) => "player-2",
            EnergyTie(s) => "tie",
            P1Turns(s) => "player-1",
            P2Turns(s) => "player-2",
            TurnTie(s) => "tie",
        }
    }

    pub fn victor(&self) -> Option<Player> {
        use EndState::*;

        match *self {
            P1Victory(s) => Some(Player::P1),
            P2Victory(s) => Some(Player::P2),
            P1Pin(s) => Some(Player::P1),
            P2Pin(s) => Some(Player::P2),
            P1Survive(s) => Some(Player::P1),
            P2Survive(s) => Some(Player::P2),
            P1Energy(s) => Some(Player::P1),
            P2Energy(s) => Some(Player::P2),
            EnergyTie(s) => None,
            P1Turns(s) => Some(Player::P1),
            P2Turns(s) => Some(Player::P2),
            TurnTie(s) => None,
        }
    }

    pub fn explain(&self) -> &'static str {
        use EndState::*;

        match *self {
            P1Victory(s) => "victory",
            P2Victory(s) => "victory",
            P1Pin(s) => "opponent-pinned",
            P2Pin(s) => "opponent-pinned",
            P1Survive(s) => "oponent-out-of-energy",
            P2Survive(s) => "opponent-out-of-energy",
            P1Energy(s) => "more-energy",
            P2Energy(s) => "more-energy",
            EnergyTie(s) => "out-of-energy",
            P1Turns(s) => "more-energy(out-of-turns)",
            P2Turns(s) => "more-energy(out-of-turns)",
            TurnTie(s) => "out-of-turns",
        }
    }
}

impl NextGameState {
    pub fn assert(&self) {
        match self {
            NextGameState::Active(ref s) => s.assert(),
            NextGameState::End(ref s) => s.assert(),
        }
    }
}

impl DecisionState {
    pub fn assert(&self) {
        let dist_sum = self.p1_dist_from_wall
            .checked_add(self.p2_dist_from_wall)
            .expect("dist_sum")
            .checked_add(self.separation_dist)
            .expect("dist_sum");
        assert_eq!(dist_sum, GAME_FIELD_SIZE);

        assert!(self.p1_energy > 0);
        assert!(self.p2_energy > 0);
    }
}

impl MoveKind {
    pub fn as_str(&self) -> &'static str {
        use MoveKind::*;
        match self {
            Back => "back",
            Stand => "stand",
            Forward => "forward",
            Lunge => "lunge",
            NoEnergy => "empty",
        }
    }
}
