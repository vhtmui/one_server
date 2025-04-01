use crossterm_ui::menu_lib;
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

        let default_selected = vec![2]; // 默认选中第1和第3个选项

        let mut menu = menu_lib::MultiSelectMenu::new(options, default_selected);

        let timer = vec![1, 2, 3, 4, 5];

        let timer_future = async {
            for i in 0..timer.len() {
                println!("Timer: {}", timer[i]);
                smol::Timer::after(std::time::Duration::from_secs(1)).await;
            }
        };

        let menu_future = async {
            let result = menu.run().await;
            println!("{result:?}");
        };
        futures::join!(timer_future, menu_future);
    }
);
