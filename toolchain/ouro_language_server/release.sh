cargo build --bin ouro_language_server -r

RELEASE_LOC="$(dirname "${BASH_SOURCE[0]}")/../../target/release/ouro_language_server"

cp $RELEASE_LOC '/Users/quinn/Library/Application Support/Zed/languages/ouro_language_server'
