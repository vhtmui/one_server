use tui_components;
use futures;
use smol;
use smol_macros::main;

main!(
    async fn main() {
        let options = vec![
            "Option 1".to_string(),
            "Option 2".to_string(),
            "Option 3".to_string(),
        ];
    }
);
