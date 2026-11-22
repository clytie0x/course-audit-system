//! One collapsible category in the Courses tab, with its course table.

use crate::components::status::{fmt_credits, label};
use crate::components::ui::{grade_class, TAG, TAG_MUTED, TAG_OK, TAG_WARN};
use audit_core::models::Category;
use leptos::prelude::*;

use crate::app::AppState;

#[component]
pub fn CategoryCard(category: Category) -> impl IntoView {
    let AppState { is_thai, .. } = expect_context::<AppState>();
    let name = category.name.clone();
    let (collected, required) = (category.collected_credits, category.required_credits);
    let overflow = category.overflow_credits();
    let complete = category.is_complete();
    let gap = required - category.counted_credits();
    let courses = category.courses.clone();
    let n = courses.len();

    view! {
        <details class="group">
            <summary class="list-none cursor-pointer grid grid-cols-[20px_minmax(0,1fr)_auto] gap-3 items-center px-4 sm:px-[18px] py-3.5 hover:bg-zinc-50 dark:hover:bg-zinc-800/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-accent-500 [&::-webkit-details-marker]:hidden">
                <svg class="w-4 h-4 text-zinc-400 transition-transform group-open:rotate-90" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="m9 6 6 6-6 6"/></svg>
                <div class="min-w-0">
                    <p class="font-semibold text-[15px] text-zinc-900 dark:text-zinc-100 truncate">{move || label(&name, is_thai.get())}</p>
                    <p class="text-[13px] text-zinc-500 dark:text-zinc-400 tabular-nums">
                        {move || if is_thai.get() {
                            format!("{} จาก {} หน่วยกิต · {} วิชา", fmt_credits(collected), fmt_credits(required), n)
                        } else {
                            format!("{} of {} credits · {} courses", fmt_credits(collected), fmt_credits(required), n)
                        }}
                    </p>
                </div>
                {if overflow > 0.0 {
                    view! { <span class=format!("{TAG} {TAG_MUTED}")>{move || if is_thai.get() { format!("เกิน +{}", fmt_credits(overflow)) } else { format!("+{} over", fmt_credits(overflow)) }}</span> }.into_any()
                } else if complete {
                    view! { <span class=format!("{TAG} {TAG_OK}")>{move || if is_thai.get() { "ครบ" } else { "Done" }}</span> }.into_any()
                } else {
                    view! { <span class=format!("{TAG} {TAG_WARN}")>{move || if is_thai.get() { format!("ขาด {}", fmt_credits(gap)) } else { format!("{} short", fmt_credits(gap)) }}</span> }.into_any()
                }}
            </summary>
            {if courses.is_empty() {
                view! {
                    <p class="px-4 sm:px-[18px] py-5 text-center text-[13px] text-zinc-500 dark:text-zinc-400 border-t border-zinc-200 dark:border-zinc-800">
                        {move || if is_thai.get() { "ยังไม่มีวิชาที่นับในหมวดนี้" } else { "No courses count toward this category yet" }}
                    </p>
                }.into_any()
            } else {
                view! {
                    <div class="overflow-x-auto">
                        <table class="w-full text-sm">
                            <thead>
                                <tr class="bg-zinc-50 dark:bg-zinc-900/60 text-left text-[12.5px] text-zinc-500 dark:text-zinc-400">
                                    <th class="font-medium px-4 sm:px-[18px] py-2 border-y border-zinc-200 dark:border-zinc-800 whitespace-nowrap">{move || if is_thai.get() { "รหัส" } else { "Code" }}</th>
                                    <th class="font-medium px-2 py-2 border-y border-zinc-200 dark:border-zinc-800">{move || if is_thai.get() { "ชื่อวิชา" } else { "Course" }}</th>
                                    <th class="hidden sm:table-cell font-medium px-2 py-2 border-y border-zinc-200 dark:border-zinc-800">{move || if is_thai.get() { "เทอม" } else { "Term" }}</th>
                                    <th class="font-medium px-2 py-2 border-y border-zinc-200 dark:border-zinc-800 text-right">{move || if is_thai.get() { "นก." } else { "Cr" }}</th>
                                    <th class="font-medium px-4 sm:px-[18px] py-2 border-y border-zinc-200 dark:border-zinc-800 text-right">{move || if is_thai.get() { "เกรด" } else { "Grade" }}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {courses.into_iter().map(|c| {
                                    let ip = c.grade.eq_ignore_ascii_case("IP");
                                    view! {
                                        <tr class="border-b last:border-b-0 border-zinc-200 dark:border-zinc-800 align-top">
                                            <td class="px-4 sm:px-[18px] py-2.5 font-mono text-[0.86em] text-zinc-500 dark:text-zinc-400 whitespace-nowrap">{c.code.clone()}</td>
                                            <td class="px-2 py-2.5 text-zinc-800 dark:text-zinc-200">{c.name.clone()}</td>
                                            <td class="hidden sm:table-cell px-2 py-2.5 text-zinc-400 tabular-nums whitespace-nowrap">{c.semester.clone().unwrap_or_default()}</td>
                                            <td class="px-2 py-2.5 text-right tabular-nums text-zinc-700 dark:text-zinc-300">{if ip { "—".to_string() } else { fmt_credits(c.credit) }}</td>
                                            <td class="px-4 sm:px-[18px] py-2.5 text-right"><span class=grade_class(&c.grade)>{c.grade.clone()}</span></td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            }}
        </details>
    }
}
