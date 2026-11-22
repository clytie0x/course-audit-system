//! Sticky top bar: brand, programme, file chip with "Change file", language
//! and dark-mode toggles.

use leptos::prelude::*;

use crate::app::AppState;
use crate::components::empty_state::reset_to_empty;

#[component]
pub fn Header() -> impl IntoView {
    let state = expect_context::<AppState>();
    let AppState {
        is_thai,
        set_is_thai,
        audit_result,
        file_name,
        ..
    } = state;
    // Dark-mode state. Initial value is sourced from the `dark` class applied
    // by the pre-paint script in index.html (honoring localStorage + system).
    let initial_dark = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
        .map(|el| el.class_list().contains("dark"))
        .unwrap_or(false);
    let (is_dark, set_is_dark) = signal(initial_dark);

    let toggle_dark = move |_| {
        set_is_dark.update(|v| *v = !*v);
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(el) = doc.document_element() {
                let list = el.class_list();
                let want_dark = is_dark.get_untracked();
                if want_dark {
                    let _ = list.add_1("dark");
                } else {
                    let _ = list.remove_1("dark");
                }
            }
            if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten())
            {
                let _ = storage.set_item(
                    "theme",
                    if is_dark.get_untracked() {
                        "dark"
                    } else {
                        "light"
                    },
                );
            }
        }
    };

    let icon_btn = "w-9 h-9 grid place-items-center rounded-[9px] border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 text-zinc-700 dark:text-zinc-300 hover:bg-zinc-50 dark:hover:bg-zinc-800 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-950";

    view! {
        <header class="sticky top-0 z-50 border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50/80 dark:bg-zinc-950/80 backdrop-blur-md backdrop-saturate-150 print:hidden">
            <div class="max-w-[1200px] mx-auto h-[60px] px-3.5 sm:px-5 flex items-center gap-2 sm:gap-3">
                <div class="flex items-center gap-2.5 shrink-0">
                    <div class="w-[30px] h-[30px] rounded-lg bg-zinc-900 dark:bg-zinc-50 grid place-items-center">
                        <svg class="w-[17px] h-[17px] text-white dark:text-zinc-900" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M22 10 12 5 2 10l10 5 10-5Z"/><path d="M6 12v5c3 2 9 2 12 0v-5"/></svg>
                    </div>
                    <span class="text-[15.5px] font-semibold text-zinc-900 dark:text-zinc-100 whitespace-nowrap">"Course Audit"</span>
                </div>
                <span class="hidden min-[860px]:inline text-[12.5px] text-zinc-600 dark:text-zinc-400 border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 rounded-full px-2.5 py-0.5 whitespace-nowrap">
                    {move || if is_thai.get() { "วิทยาการคอมพิวเตอร์ · ม.อ." } else { "Computer Science · PSU" }}
                </span>
                <div class="flex-1"></div>
                {move || audit_result.get().is_some().then(|| view! {
                    <div class="flex items-center gap-2 h-9 pl-3 pr-1.5 rounded-[9px] border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 text-[13px] min-w-0">
                        <span class="w-[7px] h-[7px] rounded-full bg-emerald-600 dark:bg-emerald-400 shrink-0" aria-hidden="true"></span>
                        <span class="hidden sm:block truncate max-w-[190px] text-zinc-700 dark:text-zinc-300">
                            {move || {
                                let name = file_name.get();
                                if !name.is_empty() {
                                    name
                                } else if is_thai.get() {
                                    "ผลที่บันทึกไว้".to_string()
                                } else {
                                    "Saved result".to_string()
                                }
                            }}
                        </span>
                        <button
                            type="button"
                            class="h-[26px] whitespace-nowrap rounded-md bg-zinc-100 dark:bg-zinc-800 px-2.5 text-[12.5px] text-zinc-600 dark:text-zinc-300 hover:text-zinc-900 dark:hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500"
                            on:click=move |_| reset_to_empty(state)
                        >
                            {move || if is_thai.get() { "เปลี่ยนไฟล์" } else { "Change file" }}
                        </button>
                    </div>
                })}
                <button
                    class=icon_btn
                    aria-label={move || if is_thai.get() { "Switch to English" } else { "เปลี่ยนเป็นภาษาไทย" }}
                    on:click=move |_| set_is_thai.update(|v| *v = !*v)
                >
                    <span class="text-[12.5px] font-semibold">{move || if is_thai.get() { "EN" } else { "ไทย" }}</span>
                </button>
                <button
                    class=icon_btn
                    aria-label={move || if is_dark.get() {
                        if is_thai.get() { "เปลี่ยนเป็นโหมดสว่าง" } else { "Switch to light mode" }
                    } else if is_thai.get() {
                        "เปลี่ยนเป็นโหมดมืด"
                    } else {
                        "Switch to dark mode"
                    }}
                    on:click=toggle_dark
                >
                    {move || if is_dark.get() {
                        view! {
                            <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m6.364.386l-1.591 1.591M21 12h-2.25m-.386 6.364l-1.591-1.591M12 18.75V21m-4.773-4.227l-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0z"/>
                            </svg>
                        }.into_any()
                    } else {
                        view! {
                            <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/></svg>
                        }.into_any()
                    }}
                </button>
            </div>
        </header>
    }
}
