# Maintaining this repository

At the moment this repository does not have good automation for maintenance. Do these actions regularly.

1. Check the open PRs and merge those that pass tests.
2. Update transitive dependencies:

    1. Check out main branch
    2. Create new branch
    3. Run `cargo update`
    4. Commit changes
    5. Push the branch
    6. Create a PR
    7. Get reviews
    8. Merge the PR.
