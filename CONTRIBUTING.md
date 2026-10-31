# Contributing to EH3D-LASER-CAM

Thank you for looking. A laser burns what it is told to, so a wrong power or a wrong position is worse than a refused job.

1. **Never panic and never guess.** What cannot be understood is an error that names the value and the reason.
2. **A change comes with a test**, and a test with damaged input when the change reads a file.
3. **Before a pull request** run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `python -I tools/ci_validate.py`: all must pass.
4. **The README exists in eight languages** (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese): a change to the text is made in all eight files.
5. Commits are written in English.
