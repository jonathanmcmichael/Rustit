//! AEC software is allowed to have a pulse.

use uuid::Uuid;

const DUNGEON_CRAWLER_CARL_CODENAME_COUNT: usize = 10;

// Dungeon Crawler Carl gets the deterministic first slots. Older favorites
// remain in the rotation, but the dungeon has naming priority.
const PROJECT_CODENAMES: [&str; 17] = [
    "Princess Donut",
    "Carl",
    "Mongo",
    "Mordecai",
    "The Royal Court",
    "Safe Room",
    "Dungeon Floor",
    "Crawler Number One",
    "Borant",
    "Achievement Unlocked",
    "Space King",
    "Aqua Teen Hunger Force",
    "Sassy the Sasquatch",
    "Big Lez",
    "Mike Nolan",
    "Clarence",
    "Donny",
];

pub(crate) fn project_codename(project_id: Uuid) -> &'static str {
    let selector = usize::from(project_id.as_bytes()[15]);
    let index = if selector % 4 != 3 {
        selector % DUNGEON_CRAWLER_CARL_CODENAME_COUNT
    } else {
        DUNGEON_CRAWLER_CARL_CODENAME_COUNT
            + (selector / 4) % (PROJECT_CODENAMES.len() - DUNGEON_CRAWLER_CARL_CODENAME_COUNT)
    };
    PROJECT_CODENAMES[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dungeon_crawler_carl_has_first_priority_in_the_codename_rotation() {
        let codename = project_codename(Uuid::nil());

        assert!(PROJECT_CODENAMES.contains(&codename));
        assert_eq!(codename, "Princess Donut");
        assert_eq!(DUNGEON_CRAWLER_CARL_CODENAME_COUNT, 10);
        assert_eq!(PROJECT_CODENAMES.len(), 17);
        assert_eq!(PROJECT_CODENAMES[1], "Carl");
        assert!(PROJECT_CODENAMES.contains(&"Mongo"));
        assert!(PROJECT_CODENAMES.contains(&"Mordecai"));
        assert!(PROJECT_CODENAMES.contains(&"The Royal Court"));
        assert!(PROJECT_CODENAMES.contains(&"Big Lez"));
        assert!(PROJECT_CODENAMES.contains(&"Mike Nolan"));

        let dcc_assignments = (0_u8..=u8::MAX)
            .filter(|selector| {
                let mut bytes = [0; 16];
                bytes[15] = *selector;
                let assigned = project_codename(Uuid::from_bytes(bytes));
                PROJECT_CODENAMES[..DUNGEON_CRAWLER_CARL_CODENAME_COUNT].contains(&assigned)
            })
            .count();
        assert_eq!(dcc_assignments, 192);
    }
}
