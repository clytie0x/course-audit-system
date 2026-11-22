//! Donut progress card per audit category.

use audit_core::models::AuditResult;
use leptos::prelude::*;

use crate::app::AppState;

#[component]
pub fn CategorySummary(result: AuditResult) -> impl IntoView {
    let AppState { is_thai, .. } = expect_context::<AppState>();

    view! {
        // ── Category Progress Cards ─────────────
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 stagger-in">
            {result.categories.iter().map(|cat| {
                let pct = ((cat.collected_credits / cat.required_credits) * 100.0).min(100.0);
                let complete = pct >= 100.0;
                let cat_name_str = cat.name.clone();
                let cat_display = {
                    let cat_name_str = cat_name_str.clone();
                    move || {
                        let is_thai = is_thai.get();
                        match cat_name_str.as_str() {
                            "General Education" if is_thai => "หมวดวิชาศึกษาทั่วไป".to_string(),
                            "Major Courses" if is_thai => "หมวดวิชาเฉพาะ".to_string(),
                            "Free Electives" if is_thai => "หมวดวิชาเลือกเสรี".to_string(),
                            _ => cat_name_str.clone(),
                        }
                    }
                };
                let collected = cat.collected_credits;
                let required = cat.required_credits;
                let pct_display = pct as u32;

                // SVG donut params
                let circumference = 100.0;
                let dash = (pct / 100.0) * circumference;

                let color_class = if complete { "text-emerald-500" } else { "text-brand-500" };
                let bg_class = if complete { "bg-emerald-50 border-emerald-100 dark:bg-emerald-950/30 dark:border-emerald-900/40" } else { "bg-white border-zinc-200/80 dark:bg-zinc-900 dark:border-zinc-800" };
                let badge_class = if complete { "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-300" } else { "bg-brand-50 text-brand-600 dark:bg-brand-950/40 dark:text-brand-300" };
                let badge_text = move || if is_thai.get() {
                    if complete { "เสร็จสมบูรณ์" } else { "กำลังดำเนินการ" }
                } else {
                    if complete { "Complete" } else { "In Progress" }
                };

                view! {
                    <div class={format!("rounded-2xl border shadow-soft p-5 flex flex-col gap-4 transition-shadow hover:shadow-medium {}", bg_class)}>
                        <div class="flex items-start justify-between">
                            <p class="text-sm font-semibold text-zinc-800 dark:text-zinc-200">{cat_display}</p>
                            <span class={format!("text-2xs font-semibold px-2 py-0.5 rounded-full {}", badge_class)}>{badge_text}</span>
                        </div>
                        <div class="flex items-center gap-4">
                            // Donut chart
                            <div class="relative w-14 h-14 shrink-0">
                                <svg class="w-14 h-14 -rotate-90" viewBox="0 0 36 36" role="img" aria-label={format!("{}% complete", pct_display)}>
                                    <circle cx="18" cy="18" r="15.9155" fill="none" class="stroke-zinc-200 dark:stroke-zinc-700" stroke-width="3"/>
                                    <circle cx="18" cy="18" r="15.9155" fill="none"
                                        class={format!("{} donut-animated", color_class)}
                                        stroke="currentColor" stroke-width="3" stroke-linecap="round"
                                        stroke-dasharray={format!("{} {}", dash, circumference - dash)}/>
                                </svg>
                                <div class="absolute inset-0 flex items-center justify-center">
                                    <span class="text-xs font-bold text-zinc-700 dark:text-zinc-200" aria-hidden="true">{format!("{}%", pct_display)}</span>
                                </div>
                            </div>
                            // Credits
                            <div>
                                <div class="flex items-baseline gap-1">
                                    <span class="text-2xl font-bold text-zinc-900 dark:text-zinc-100 tabular-nums">{collected as u32}</span>
                                    <span class="text-sm text-zinc-400 dark:text-zinc-500 font-medium">{format!("/ {}", required as u32)}</span>
                                </div>
                                <p class="text-2xs text-zinc-400 dark:text-zinc-500 mt-0.5 font-medium">{move || if is_thai.get() { "หน่วยกิต" } else { "credits" }}</p>
                            </div>
                        </div>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
