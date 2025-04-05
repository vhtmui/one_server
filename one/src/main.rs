use crossterm_ui::component::Selection;
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

        let mut menu = Selection::new_with_default(options);

        let result = menu.run().await;
        println!("{result:?}");
    }
);
