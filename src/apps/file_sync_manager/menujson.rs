pub const MENU_JSON: &str = r#"
{
    "name": "Monitor Menu",
    "content": "This is a menu of file monitor.",
    "children": [
        {
            "name": "monitor",
            "content": "This is a description.",
            "children": [
                {
                    "name": "start",
                    "content": "This is a description of Skyrim.",
                    "children": []
                },
                {
                    "name": "stop",
                    "content": "This is a description of Skyrim.",
                    "children": []
                }
            ]
        },
        {
            "name": "scanner",
            "content": "This is a description of scanner.",
            "children": [
                {
                    "name": "start",
                    "content": "This is a description of Skyrim.",
                    "children": []
                },
                {
                    "name": "start_periodic",
                    "content": "Start periodic scan.",
                    "children": []
                },
                {
                    "name": "stop",
                    "content": "Stop periodic scan.",
                    "children": []

                }
            ]
        }
    ]
}
"#;