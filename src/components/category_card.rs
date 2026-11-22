//! Category Card Component
//!
//! Displays an expandable/collapsible category with progress tracking.
//! Minimal, clean design with smooth expand/collapse.

use audit_core::models::Category;
use leptos::*;

/// Collapsible accordion row for a course category
#[component]
pub fn CategoryCard(category: Category) -> impl IntoView {
    let (is_expanded, set_is_expanded) = create_signal(false);
    let percentage = (category.collected_credits / category.required_credits * 100.0).min(100.0);
    let complete = percentage >= 100.0;
    let category_clone = category.clone();

    let progress_color = if complete {
        "bg-emerald-500"
    } else {
        "bg-brand-500"
    };

    let panel_id = format!(
        "category-panel-{}",
        category
            .name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    );
    let panel_id_attr = panel_id.clone();

    view! {
        <div class="group">
            // Header row
            <button
                class="w-full px-5 py-3.5 flex items-center justify-between hover:bg-zinc-50/80 dark:hover:bg-zinc-800/50 transition-colors text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-inset"
                aria-expanded={move || if is_expanded.get() { "true" } else { "false" }}
                aria-controls=panel_id_attr
                on:click=move |_| set_is_expanded.update(|v| *v = !*v)
            >
                <div class="flex items-center gap-3 min-w-0">
                    // Expand icon
                    <div class="shrink-0">
                        <svg
                            class={move || format!(
                                "w-4 h-4 text-zinc-400 dark:text-zinc-500 transition-transform duration-200 {}",
                                if is_expanded.get() { "rotate-90" } else { "" }
                            )}
                            aria-hidden="true"
                            fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"
                        >
                            <path stroke-linecap="round" stroke-linejoin="round" d="M8.25 4.5l7.5 7.5-7.5 7.5" />
                        </svg>
                    </div>
                    <span class="text-sm font-medium text-zinc-800 dark:text-zinc-200 truncate">{
                        let name = category.name.clone();
                        move || {
                            let is_thai = use_context::<ReadSignal<bool>>().map(|s| s.get()).unwrap_or(false);
                            match name.as_str() {
                                "General Education" if is_thai => "หมวดวิชาศึกษาทั่วไป".to_string(),
                                "Major Courses" if is_thai => "หมวดวิชาเฉพาะ".to_string(),
                                "Free Electives" if is_thai => "หมวดวิชาเลือกเสรี".to_string(),
                                _ => name.clone(),
                            }
                        }
                    }</span>
                    {if complete {
                        view! {
                            <svg class="w-4 h-4 text-emerald-500 shrink-0" aria-hidden="true" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.857-9.809a.75.75 0 00-1.214-.882l-3.483 4.79-1.88-1.88a.75.75 0 10-1.06 1.061l2.5 2.5a.75.75 0 001.137-.089l4-5.5z" clip-rule="evenodd"/>
                            </svg>
                        }.into_view()
                    } else {
                        view! { <span></span> }.into_view()
                    }}
                </div>
                <div class="flex items-center gap-3 shrink-0 ml-4">
                    // Mini progress bar
                    <div class="hidden sm:block w-20 bg-zinc-100 dark:bg-zinc-800 rounded-full h-1 overflow-hidden">
                        <div
                            class={format!("h-full rounded-full progress-animated {}", progress_color)}
                            style={format!("width: {}%", percentage)}
                        ></div>
                    </div>
                    <span class="text-xs font-mono font-medium text-zinc-500 dark:text-zinc-400 tabular-nums w-16 text-right">
                        {format!("{:.0}/{:.0} cr", category.collected_credits, category.required_credits)}
                    </span>
                </div>
            </button>

            // Expanded course list
            {move || {
                if is_expanded.get() {
                    view! {
                        <div id=panel_id.clone() class="border-t border-zinc-100 dark:border-zinc-800 bg-zinc-50/40 dark:bg-zinc-900/60 animate-fade-in">
                            {if category_clone.courses.is_empty() {
                                view! {
                                    <div class="px-5 py-6 text-center">
                                        <p class="text-xs text-zinc-400 dark:text-zinc-500 font-medium">{move || {
                                            let is_thai = use_context::<ReadSignal<bool>>().map(|s| s.get()).unwrap_or(false);
                                            if is_thai { "ยังไม่มีวิชาที่เรียนสำเร็จในหมวดนี้" } else { "No courses completed in this category" }
                                        }}</p>
                                    </div>
                                }.into_view()
                            } else {
                                view! {
                                    <div class="divide-y divide-zinc-100/80 dark:divide-zinc-800/80">
                                        {category_clone.courses.iter().map(|course| {
                                            let course = course.clone();
                                            let is_ip = course.grade.eq_ignore_ascii_case("IP");
                                            let grade_color = if is_ip {
                                                "bg-amber-50 text-amber-700 border-amber-200/60 dark:bg-amber-950/40 dark:text-amber-300 dark:border-amber-900/50"
                                            } else {
                                                match course.grade.chars().next().unwrap_or('F') {
                                                    'A' => "bg-emerald-50 text-emerald-700 border-emerald-200/60 dark:bg-emerald-950/40 dark:text-emerald-300 dark:border-emerald-900/50",
                                                    'B' => "bg-blue-50 text-blue-700 border-blue-200/60 dark:bg-blue-950/40 dark:text-blue-300 dark:border-blue-900/50",
                                                    'C' => "bg-amber-50 text-amber-700 border-amber-200/60 dark:bg-amber-950/40 dark:text-amber-300 dark:border-amber-900/50",
                                                    'D' => "bg-orange-50 text-orange-700 border-orange-200/60 dark:bg-orange-950/40 dark:text-orange-300 dark:border-orange-900/50",
                                                    _ => "bg-zinc-50 text-zinc-600 border-zinc-200 dark:bg-zinc-800/60 dark:text-zinc-300 dark:border-zinc-700",
                                                }
                                            };
                                            let credit_cell = if is_ip {
                                                "—".to_string()
                                            } else {
                                                format!("{}", course.credit as u32)
                                            };
                                            view! {
                                                <div class="flex items-center justify-between px-5 py-2.5 hover:bg-white/60 dark:hover:bg-zinc-800/40 transition-colors">
                                                    <div class="flex items-center gap-3 min-w-0 flex-1">
                                                        <span class="font-mono text-2xs font-semibold text-zinc-400 dark:text-zinc-500 w-14 shrink-0">{&course.code}</span>
                                                        <span class="text-[13px] text-zinc-700 dark:text-zinc-300 truncate">{&course.name}</span>
                                                    </div>
                                                    <div class="flex items-center gap-2.5 shrink-0 ml-3">
                                                        <span class={format!("text-2xs font-bold min-w-[1.75rem] h-5 px-1 flex items-center justify-center rounded border {}", grade_color)}>
                                                            {&course.grade}
                                                        </span>
                                                        <span class="text-2xs text-zinc-400 dark:text-zinc-500 font-mono w-6 text-right">{credit_cell}</span>
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_view()
                            }}
                        </div>
                    }.into_view()
                } else {
                    view! { <div></div> }.into_view()
                }
            }}
        </div>
    }
}
