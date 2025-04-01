use smol;
use futures;
use crossterm_ui::menu_lib;

fn main() {
    let options = vec![
        "Option 1".to_string(),
        "Option 2".to_string(),
        "Option 3".to_string(),
    ];

    let default_selected = vec![2]; // 默认选中第1和第3个选项

    let mut menu = menu_lib::MultiSelectMenu::new(options, default_selected);

    let timer = vec![1,2,3,4,5];

    let timing = async {
        for i in 0..timer.len() {
            println!("Timer: {}", timer[i]);
            smol::Timer::after(std::time::Duration::from_secs(1)).await;
        }
    };

    smol::block_on(async {
        let timing_future = timing;
        let menu_future = async {
            let result = menu.run().await;
            println!("{result:?}");
        };
        futures::join!(timing_future, menu_future);
    });
}
