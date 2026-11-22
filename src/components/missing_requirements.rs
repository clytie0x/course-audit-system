//! Missing requirements grouped by category, with copy-to-clipboard.

use audit_core::models::AuditResult;
use leptos::prelude::*;

use crate::app::AppState;

#[component]
pub fn MissingRequirements(result: AuditResult) -> impl IntoView {
    let AppState {
        audit_result,
        is_thai,
        ..
    } = expect_context::<AppState>();
    // Format the current result's missing list as a newline-delimited block
    // and copy to clipboard (Navigator.clipboard.writeText).
    let copy_missing = move |_| {
        let Some(result) = audit_result.get_untracked() else {
            return;
        };
        if result.missing_subjects.is_empty() {
            return;
        }
        let mut out = String::new();
        for m in &result.missing_subjects {
            out.push_str(&format!("[{}] {}\n", m.category, m.description));
        }
        if let Some(clipboard) = web_sys::window().map(|w| w.navigator().clipboard()) {
            let _ = clipboard.write_text(&out);
        }
    };

    view! {
        // ── Missing Requirements ─────────────────
        {(!result.missing_subjects.is_empty()).then(|| {
            let mut seen_cats: Vec<String> = Vec::new();
            for m in &result.missing_subjects {
                if !seen_cats.contains(&m.category) {
                    seen_cats.push(m.category.clone());
                }
            }
            let missing_count = result.missing_subjects.len();
            view! {
                <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-red-200/60 dark:border-red-900/40 shadow-soft overflow-hidden">
                    <div class="px-5 py-4 border-b border-red-100 dark:border-red-900/40 flex items-center gap-2.5 bg-red-50/50 dark:bg-red-950/20">
                        <svg class="w-4 h-4 text-red-500" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z"/></svg>
                        <h3 class="text-sm font-semibold text-red-800 dark:text-red-300">{move || if is_thai.get() { "ข้อกำหนดที่ขาด" } else { "Missing Requirements" }}</h3>
                        <span class="ml-auto text-2xs font-semibold text-red-600 dark:text-red-300 bg-red-100 dark:bg-red-900/40 px-2 py-0.5 rounded-full">{move || if is_thai.get() { format!("{} รายการ", missing_count) } else { format!("{} items", missing_count) }}</span>
                        <button
                            class="print:hidden inline-flex items-center gap-1 text-2xs font-semibold text-red-700 dark:text-red-300 hover:text-red-900 dark:hover:text-red-100 border border-red-200 dark:border-red-900/50 hover:bg-red-100 dark:hover:bg-red-900/30 px-2 py-0.5 rounded-md transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-500"
                            aria-label={move || if is_thai.get() { "คัดลอกรายการที่ขาด" } else { "Copy missing items" }}
                            on:click=copy_missing
                        >
                            <svg class="w-3 h-3" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M15.75 17.25v3.375c0 .621-.504 1.125-1.125 1.125h-9.75a1.125 1.125 0 01-1.125-1.125V7.875c0-.621.504-1.125 1.125-1.125H6.75a9.06 9.06 0 011.5.124m7.5 10.376h3.375c.621 0 1.125-.504 1.125-1.125V11.25c0-4.46-3.243-8.161-7.5-8.876a9.06 9.06 0 00-1.5-.124H9.375c-.621 0-1.125.504-1.125 1.125v3.5m7.5 10.375H9.375a1.125 1.125 0 01-1.125-1.125v-9.25m12 6.625v-1.875a3.375 3.375 0 00-3.375-3.375h-1.5a1.125 1.125 0 01-1.125-1.125v-1.5a3.375 3.375 0 00-3.375-3.375H9.75"/></svg>
                            <span>{move || if is_thai.get() { "คัดลอก" } else { "Copy" }}</span>
                        </button>
                    </div>
                    <div class="divide-y divide-red-100/60 dark:divide-red-900/30">
                        {seen_cats.iter().map(|cat| {
                            let cat_courses: Vec<_> = result.missing_subjects.iter()
                                .filter(|m| &m.category == cat)
                                .collect();
                            let display_items: Vec<String> = if cat == "General Education" {
                                let mut ge_groups: Vec<String> = Vec::new();
                                for m in &cat_courses {
                                    let description = m.description.trim();
                                    let group = if description.contains("missing") {
                                        description.to_string()
                                    } else {
                                        description.split(':').next().unwrap_or(description).trim().to_string()
                                    };
                                    if !ge_groups.contains(&group) {
                                        ge_groups.push(group);
                                    }
                                }
                                ge_groups
                            } else {
                                cat_courses.iter().map(|m| m.description.clone()).collect()
                            };
                            let cat_display_name = {
                                let name = cat.clone();
                                move || {
                                    let is_thai = is_thai.get();
                                    match name.as_str() {
                                        "General Education" if is_thai => "หมวดวิชาศึกษาทั่วไป".to_string(),
                                        "Major Courses" if is_thai => "หมวดวิชาเฉพาะ".to_string(),
                                        "Free Electives" if is_thai => "หมวดวิชาเลือกเสรี".to_string(),
                                        _ => name.clone(),
                                    }
                                }
                            };
                            view! {
                                <div class="p-5">
                                    <p class="text-xs font-semibold text-zinc-700 dark:text-zinc-300 uppercase tracking-wider mb-2.5">{cat_display_name}</p>
                                    <div class="space-y-1.5">
                                        {display_items.iter().map(|item| {
                                            let desc = item.clone();
                                            view! {
                                                <div class="flex items-start gap-2.5 py-1.5">
                                                    <div class="w-1.5 h-1.5 rounded-full bg-red-400 dark:bg-red-500 mt-1.5 shrink-0"></div>
                                                    <p class="text-[13px] text-zinc-600 dark:text-zinc-300 leading-relaxed">{desc}</p>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            }
        })}
    }
}
