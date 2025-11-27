use dioxus::prelude::*;

#[component]
pub fn Navbar() -> Element {
    rsx! {
        nav {
            style: "
                width: 100%;
                min-height: 60px;
                max-height: 60px;
                background: var(--sidebar);
                border-bottom: 1px solid var(--border);
                display: flex;
                align-items: center;
                justify-content: space-between;
                padding: 0 24px;
                box-sizing: border-box;
                flex-shrink: 0;
            ",
            div {
                style: "
                    font-size: clamp(1rem, 2vw, 1.5rem);
                    font-weight: 600;
                    color: var(--foreground);
                    white-space: nowrap;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    margin-right: 16px;
                ",
                "📜 Claude Code Viewer"
            }
        }
    }
}
