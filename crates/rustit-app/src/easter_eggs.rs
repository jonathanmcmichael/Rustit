//! AEC software is allowed to have a pulse.

use uuid::Uuid;

const PROJECT_CODENAMES: [&str; 3] = [
    "Space King",
    "Aqua Teen Hunger Force",
    "Sassy the Sasquatch",
];

pub(crate) fn project_codename(project_id: Uuid) -> &'static str {
    let index = usize::from(project_id.as_bytes()[15]) % PROJECT_CODENAMES.len();
    PROJECT_CODENAMES[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_project_gets_one_of_the_three_required_codenames() {
        let codename = project_codename(Uuid::nil());

        assert!(PROJECT_CODENAMES.contains(&codename));
        assert_eq!(PROJECT_CODENAMES.len(), 3);
    }
}
