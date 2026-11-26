//! Sticky top bar: brand, programme, file chip with "Change file" and the
//! language toggle.

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
    let icon_btn = "w-9 h-9 grid place-items-center rounded-[9px] border border-zinc-200 bg-white text-zinc-700 hover:bg-zinc-50 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500 focus-visible:ring-offset-2";

    view! {
        <header class="sticky top-0 z-50 border-b border-zinc-200 bg-zinc-50/80 backdrop-blur-md backdrop-saturate-150 print:hidden">
            <div class="max-w-[1200px] mx-auto h-[60px] px-3.5 sm:px-5 flex items-center gap-2 sm:gap-3">
                <div class="flex items-center gap-2.5 shrink-0">
                    <div class="w-[30px] h-[30px] rounded-lg bg-zinc-900 grid place-items-center">
                        <svg class="w-[17px] h-[17px] text-white" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M22 10 12 5 2 10l10 5 10-5Z"/><path d="M6 12v5c3 2 9 2 12 0v-5"/></svg>
                    </div>
                    <span class="text-[15.5px] font-semibold text-zinc-900 whitespace-nowrap">"Course Audit"</span>
                </div>
                <span class="hidden min-[860px]:inline text-[12.5px] text-zinc-600 border border-zinc-200 bg-white rounded-full px-2.5 py-0.5 whitespace-nowrap">
                    {move || if is_thai.get() { "วิทยาการคอมพิวเตอร์ · ม.อ." } else { "Computer Science · PSU" }}
                </span>
                <div class="flex-1"></div>
                {move || audit_result.get().is_some().then(|| view! {
                    <div class="flex items-center gap-2 h-9 pl-3 pr-1.5 rounded-[9px] border border-zinc-200 bg-white text-[13px] min-w-0">
                        <span class="w-[7px] h-[7px] rounded-full bg-emerald-600 shrink-0" aria-hidden="true"></span>
                        <span class="hidden sm:block truncate max-w-[190px] text-zinc-700">
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
                            class="h-[26px] whitespace-nowrap rounded-md bg-zinc-100 px-2.5 text-[12.5px] text-zinc-600 hover:text-zinc-900 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500"
                            on:click=move |_| reset_to_empty(state)
                        >
                            {move || if is_thai.get() { "เปลี่ยนไฟล์" } else { "Change file" }}
                        </button>
                    </div>
                })}
                <button
                    type="button"
                    class=icon_btn
                    lang={move || if is_thai.get() { "en" } else { "th" }}
                    aria-label={move || if is_thai.get() { "Switch to English" } else { "เปลี่ยนเป็นภาษาไทย" }}
                    on:click=move |_| set_is_thai.update(|v| *v = !*v)
                >
                    <span class="text-[12.5px] font-semibold">{move || if is_thai.get() { "EN" } else { "ไทย" }}</span>
                </button>
            </div>
        </header>
    }
}
