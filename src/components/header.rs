//! Sticky top navbar: title, language toggle, dark-mode toggle.

use leptos::prelude::*;

use crate::app::AppState;

#[component]
pub fn Header() -> impl IntoView {
    let AppState {
        is_thai,
        set_is_thai,
        ..
    } = expect_context::<AppState>();
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

    view! {
        // ── Navbar ──────────────────────────────────────────────────
        <header class="sticky top-0 z-50 border-b border-zinc-200/60 dark:border-zinc-800/60 bg-white/80 dark:bg-zinc-950/80 backdrop-blur-xl backdrop-saturate-150">
            <div class="max-w-[1440px] mx-auto px-4 sm:px-6 lg:px-8 h-14 flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <div class="w-7 h-7 rounded-lg bg-zinc-900 dark:bg-zinc-50 flex items-center justify-center">
                        <svg class="w-4 h-4 text-white dark:text-zinc-900" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.2" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M4.26 10.147a60.436 60.436 0 00-.491 6.347A48.627 48.627 0 0112 20.904a48.627 48.627 0 018.232-4.41 60.46 60.46 0 00-.491-6.347m-15.482 0a50.57 50.57 0 00-2.658-.813A59.905 59.905 0 0112 3.493a59.902 59.902 0 0110.399 5.84c-.896.248-1.783.52-2.658.814m-15.482 0A50.697 50.697 0 0112 13.489a50.702 50.702 0 017.74-3.342"/>
                        </svg>
                    </div>
                    <span class="text-[15px] font-semibold tracking-tight text-zinc-900 dark:text-zinc-100">"Course Audit"</span>
                    <span class="hidden sm:inline text-xs font-medium text-zinc-400 dark:text-zinc-500 bg-zinc-100 dark:bg-zinc-800 px-2 py-0.5 rounded-md">"CS · PSU"</span>
                </div>
                <div class="flex items-center gap-3">
                    <button
                        class="text-xs font-semibold min-w-[2.5rem] min-h-[2.25rem] px-3 py-1.5 rounded-lg bg-zinc-100 hover:bg-zinc-200 text-zinc-600 dark:bg-zinc-800 dark:hover:bg-zinc-700 dark:text-zinc-300 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-900"
                        aria-label={move || if is_thai.get() { "Switch to English" } else { "เปลี่ยนเป็นภาษาไทย" }}
                        on:click=move |_| set_is_thai.update(|v| *v = !*v)
                    >
                        {move || if is_thai.get() { "EN" } else { "ไทย" }}
                    </button>
                    <button
                        class="w-9 h-9 rounded-lg bg-zinc-100 hover:bg-zinc-200 text-zinc-600 dark:bg-zinc-800 dark:hover:bg-zinc-700 dark:text-zinc-300 transition-colors flex items-center justify-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-900"
                        aria-label={move || if is_dark.get() {
                            if is_thai.get() { "เปลี่ยนเป็นโหมดสว่าง" } else { "Switch to light mode" }
                        } else {
                            if is_thai.get() { "เปลี่ยนเป็นโหมดมืด" } else { "Switch to dark mode" }
                        }}
                        on:click=toggle_dark
                    >
                        {move || if is_dark.get() {
                            view! {
                                <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m6.364.386l-1.591 1.591M21 12h-2.25m-.386 6.364l-1.591-1.591M12 18.75V21m-4.773-4.227l-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0z"/>
                                </svg>
                            }.into_any()
                        } else {
                            view! {
                                <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.718 9.718 0 0118 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 003 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 009.002-5.998z"/>
                                </svg>
                            }.into_any()
                        }}
                    </button>
                    <div class="flex items-center gap-2 text-xs text-zinc-400 dark:text-zinc-500">
                        <div class="w-1.5 h-1.5 rounded-full bg-emerald-400 pulse-dot"></div>
                        <span class="hidden sm:inline font-medium">{move || if is_thai.get() { "100% ประมวลผลบนเครื่อง" } else { "100% client-side" }}</span>
                    </div>
                </div>
            </div>
        </header>
    }
}
