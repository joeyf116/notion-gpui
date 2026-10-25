use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Status {
    #[default]
    Backlog,
    InProgress,
    Done,
}

impl Status {
    pub const ALL: [Self; 3] = [Self::Backlog, Self::InProgress, Self::Done];

    pub fn title(self) -> &'static str {
        match self {
            Self::Backlog => "Backlog",
            Self::InProgress => "In progress",
            Self::Done => "Done",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Backlog => Self::InProgress,
            Self::InProgress => Self::Done,
            Self::Done => Self::Backlog,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Card {
    pub id: u64,
    pub title: String,
    pub assignee: String,
    pub status: Status,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Board {
    pub cards: Vec<Card>,
}

impl Board {
    pub fn add(&mut self, title: &str, assignee: &str) -> Result<(), &'static str> {
        let title = title.trim();
        if title.is_empty() {
            return Err("Enter a task title.");
        }
        let id = self
            .cards
            .iter()
            .map(|card| card.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("The board has run out of task IDs.")?;
        self.cards.push(Card {
            id,
            title: title.to_owned(),
            assignee: assignee.trim().to_owned(),
            status: Status::Backlog,
        });
        Ok(())
    }

    pub fn advance(&mut self, id: u64) {
        if let Some(card) = self.cards.iter_mut().find(|card| card.id == id) {
            card.status = card.status.next();
        }
    }

    pub fn remove(&mut self, id: u64) {
        self.cards.retain(|card| card.id != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_normalizes_tasks() {
        let mut board = Board::default();
        assert!(board.add(" \n ", "Someone").is_err());
        assert!(board.cards.is_empty());
        board.add(" Plan release ", " Alex ").unwrap();
        assert_eq!(board.cards[0].title, "Plan release");
        assert_eq!(board.cards[0].assignee, "Alex");
        assert_eq!(board.cards[0].status, Status::Backlog);
    }

    #[test]
    fn moves_and_removes_only_the_selected_task() {
        let mut board = Board::default();
        board.add("First", "").unwrap();
        board.add("Second", "").unwrap();
        for expected in [Status::InProgress, Status::Done, Status::Backlog] {
            board.advance(1);
            assert_eq!(board.cards[0].status, expected);
            assert_eq!(board.cards[1].status, Status::Backlog);
        }
        board.advance(999);
        board.remove(1);
        board.add("Third", "").unwrap();
        assert_eq!(board.cards[0].id, 2);
        assert_eq!(board.cards[1].id, 3);
    }

    #[test]
    fn id_exhaustion_does_not_mutate_the_board() {
        let mut board = Board {
            cards: vec![Card {
                id: u64::MAX,
                title: "Existing task".into(),
                assignee: String::new(),
                status: Status::Done,
            }],
        };
        let original = board.clone();
        assert!(board.add("Another task", "").is_err());
        assert_eq!(board, original);
    }
}
