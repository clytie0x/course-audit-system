//! Results view: page head with export/print, status banner, KPI row and the
//! tabbed sections (overview, courses, semesters, plan, offerings).

use crate::components::category_card::CategoryCard;
use crate::components::coop_hours::{CoopHoursState, TOTAL_REQUIRED};
use crate::components::empty_state::SaveToggle;
use crate::components::offerings::OfferingsPanel;
use crate::components::overview::Overview;
use crate::components::semester_breakdown::SemesterBreakdown;
use crate::components::status::{fmt_credits, label, status_line};
use crate::components::ui::{BUTTON, CARD, TAG, TAG_ACCENT, TAG_OK};
use crate::components::what_if::WhatIfSimulator;
use audit_core::data::{curricula, curriculum};
use audit_core::models::AuditResult;
use audit_core::models::ParseMode;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::app::AppState;

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Overview,
    Courses,
    Semesters,
    Plan,
    Offerings,
}

impl Tab {
    const ALL: [Tab; 5] = [
        Tab::Overview,
        Tab::Courses,
        Tab::Semesters,
        Tab::Plan,
        Tab::Offerings,
    ];

    fn id(self) -> &'static str {
        match self {
            Tab::Overview => "overview",
            Tab::Courses => "courses",
            Tab::Semesters => "semesters",
            Tab::Plan => "plan",
            Tab::Offerings => "offerings",
        }
    }

    fn title(self, is_thai: bool) -> &'static str {
        match (self, is_thai) {
            (Tab::Overview, true) => "ภาพรวม",
            (Tab::Overview, false) => "Overview",
            (Tab::Courses, true) => "รายวิชา",
            (Tab::Courses, false) => "Courses",
            (Tab::Semesters, true) => "รายเทอม",
            (Tab::Semesters, false) => "Semesters",
            (Tab::Plan, true) => "จำลองแผน",
            (Tab::Plan, false) => "Plan",
            (Tab::Offerings, true) => "วิชาที่เปิดสอน",
            (Tab::Offerings, false) => "Offerings",
        }
    }
}

/// Selected tab, provided by `App` so it survives the results view being
/// rebuilt (e.g. a curriculum switch) and lets a section switch tabs.
#[derive(Clone, Copy)]
pub struct SelectedTab(pub RwSignal<Tab>);

#[component]
pub fn ResultsView(result: AuditResult) -> impl IntoView {
    let AppState {
        audit_result,
        parse_mode,
        curriculum_id,
        set_curriculum_id,
        cohort,
        is_thai,
        ..
    } = expect_context::<AppState>();
    let CoopHoursState {
        faculty,
        university,
        ..
    } = expect_context::<CoopHoursState>();
    let SelectedTab(tab) = expect_context::<SelectedTab>();

    // Download the AuditResult as a JSON file.
    let export_json = move |_| {
        let Some(result) = audit_result.get_untracked() else {
            return;
        };
        let Ok(json) = serde_json::to_string_pretty(&result) else {
            return;
        };
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        // Build a Blob containing the JSON text and stream it via <a download>.
        let parts = js_sys::Array::new();
        parts.push(&JsValue::from_str(&json));
        let Ok(blob) = web_sys::Blob::new_with_str_sequence(&parts) else {
            return;
        };
        let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
            return;
        };
        if let Ok(anchor) = document.create_element("a") {
            let _ = anchor.set_attribute("href", &url);
            let _ = anchor.set_attribute("download", "course-audit.json");
            if let Some(html_anchor) = anchor.dyn_ref::<web_sys::HtmlElement>() {
                html_anchor.click();
            }
        }
        let _ = web_sys::Url::revoke_object_url(&url);
    };

    // Trigger the browser's print dialog; style.css provides a print stylesheet.
    let print_report = move |_| {
        if let Some(window) = web_sys::window() {
            let _ = window.print();
        }
    };

    let counted = result.counted_credits();
    let required = result.required_credits();
    let earned = result.earned_credits();
    let overflow: f32 = result.categories.iter().map(|c| c.overflow_credits()).sum();
    let short = (required - counted).max(0.0);
    let pct = if required > 0.0 {
        (counted / required * 100.0).round()
    } else {
        0.0
    };
    let gpa = result.gpa;
    // Saves from before `attempts` existed only have the category lists.
    let course_count = if result.attempts.is_empty() {
        result.categories.iter().map(|c| c.courses.len()).sum()
    } else {
        result.attempts.len()
    };
    // Where the missing credits are: groups when a category has them.
    let gaps: Vec<(String, f32)> = result
        .categories
        .iter()
        .flat_map(|c| {
            if c.groups.is_empty() {
                vec![(c.name.clone(), c.required_credits - c.counted_credits())]
            } else {
                c.groups
                    .iter()
                    .map(|g| {
                        (
                            g.name.clone(),
                            g.required_credits - g.collected_credits.min(g.required_credits),
                        )
                    })
                    .collect()
            }
        })
        .filter(|(_, gap)| *gap > 0.0)
        .collect();
    let result_for_status = result.clone();
    let categories = result.categories.clone();

    let on_tab_key = move |ev: web_sys::KeyboardEvent| {
        let i = Tab::ALL
            .iter()
            .position(|t| *t == tab.get_untracked())
            .unwrap_or(0) as isize;
        let last = Tab::ALL.len() as isize - 1;
        let target = match ev.key().as_str() {
            "ArrowRight" => (i + 1).rem_euclid(last + 1),
            "ArrowLeft" => (i - 1).rem_euclid(last + 1),
            "Home" => 0,
            "End" => last,
            _ => return,
        };
        ev.prevent_default();
        let next = Tab::ALL[target as usize];
        tab.set(next);
        if let Some(el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(&format!("tab-{}", next.id())))
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = el.focus();
        }
    };

    let panel = move |t: Tab, body: AnyView| {
        view! {
            <div
                id=format!("panel-{}", t.id())
                role="tabpanel"
                aria-labelledby=format!("tab-{}", t.id())
                class="tab-panel"
                hidden=move || tab.get() != t
            >
                {body}
            </div>
        }
    };

    view! {
        <div class="animate-fade-in">
            // ── Page head ─────────────────────────────
            <div class="flex flex-wrap items-end gap-4 mb-5">
                <div class="min-w-0">
                    <h1 class="text-[22px] sm:text-[26px] leading-snug font-semibold text-zinc-900">
                        {move || if is_thai.get() { "สรุปผลการตรวจหลักสูตร" } else { "Curriculum audit summary" }}
                    </h1>
                    <div class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-zinc-600">
                        <span>{move || if is_thai.get() { "วิทยาการคอมพิวเตอร์" } else { "Computer Science" }}</span>
                        <span aria-hidden="true">"·"</span>
                        <select
                            class="bg-transparent rounded-md border border-transparent hover:border-zinc-200 pr-1 py-0.5 text-sm text-zinc-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500"
                            aria-label={move || if is_thai.get() { "หลักสูตรที่ใช้ตรวจ" } else { "Curriculum used for the audit" }}
                            prop:value=move || curriculum_id.get()
                            on:change=move |ev| set_curriculum_id.set(event_target_value(&ev))
                        >
                            {curricula().iter().map(|c| view! { <option value=c.id.clone()>{c.name.clone()}</option> }).collect::<Vec<_>>()}
                        </select>
                        {move || cohort.get().map(|c| view! {
                            <span aria-hidden="true">"·"</span>
                            <span>{move || if is_thai.get() { format!("รุ่นรหัส {}", c % 100) } else { format!("Cohort {}", c % 100) }}</span>
                        })}
                    </div>
                </div>
                <div class="flex gap-2 w-full sm:w-auto sm:ml-auto print:hidden">
                    <button class=format!("{BUTTON} flex-1 sm:flex-none justify-center") on:click=export_json>
                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M12 4v12m0 0-4-4m4 4 4-4M4 20h16"/></svg>
                        {move || if is_thai.get() { "ส่งออก JSON" } else { "Export JSON" }}
                    </button>
                    <button class=format!("{BUTTON} flex-1 sm:flex-none justify-center") on:click=print_report>
                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M7 9V3h10v6"/><rect x="3" y="9" width="18" height="8" rx="2"/><path d="M7 14h10v7H7z"/></svg>
                        {move || if is_thai.get() { "พิมพ์รายงาน" } else { "Print report" }}
                    </button>
                </div>
            </div>

            // ── Registration-mode banner ─────────────
            {move || (parse_mode.get() == ParseMode::Registration).then(|| view! {
                <div class="rounded-xl border border-amber-200/70 bg-amber-50 px-4 py-3 mb-4 flex items-start gap-3" role="status">
                    <svg class="w-5 h-5 text-amber-600 shrink-0 mt-0.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m0-10.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285zM12 15.75h.007v.008H12v-.008z"/></svg>
                    <div class="text-sm text-amber-900">
                        <p class="font-semibold">{move || if is_thai.get() { "กำลังดูไฟล์ผลการลงทะเบียน" } else { "Viewing a registration file" }}</p>
                        <p class="text-amber-800/90">{move || if is_thai.get() { "วิชาที่กำลังเรียนยังไม่นับหน่วยกิตจนกว่าจะได้เกรด" } else { "In-progress courses don't count toward credits until they're graded." }}</p>
                    </div>
                </div>
            })}

            // ── Status banner ────────────────────────
            {move || {
                let (ok, head, note) = status_line(&result_for_status, is_thai.get());
                let box_class = if ok {
                    "border-emerald-200 bg-emerald-50"
                } else {
                    "border-amber-200 bg-amber-50"
                };
                let icon_class = if ok { "text-emerald-700" } else { "text-amber-700" };
                view! {
                    <div class=format!("rounded-xl border px-4 py-3 mb-5 flex items-start gap-3 {box_class}") role="status">
                        <svg class=format!("w-5 h-5 shrink-0 mt-0.5 {icon_class}") aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24">
                            {if ok {
                                view! { <path d="M20 6 9 17l-5-5"/> }.into_any()
                            } else {
                                view! { <circle cx="12" cy="12" r="9"/><path d="M12 8v4m0 4h.01"/> }.into_any()
                            }}
                        </svg>
                        <div class="text-sm">
                            <p class="font-semibold text-zinc-900">{head}</p>
                            {note.map(|n| view! { <p class="text-zinc-600">{n}</p> })}
                        </div>
                    </div>
                }
            }}

            // ── KPI row ──────────────────────────────
            <div class="grid grid-cols-2 min-[700px]:grid-cols-3 lg:grid-cols-[1.35fr_1fr_1fr_1fr] gap-3 mb-7">
                <div class=format!("{CARD} px-4 py-4 sm:px-[18px] col-span-2 min-[700px]:col-span-3 lg:col-span-1")>
                    <p class="text-[13px] text-zinc-600">{move || if is_thai.get() { "หน่วยกิตที่นับตามหลักสูตร" } else { "Credits counted toward the curriculum" }}</p>
                    <div class="flex items-baseline gap-1.5 mt-2 mb-2.5">
                        <span class="text-[28px] sm:text-[30px] leading-tight font-semibold tabular-nums text-zinc-900">{fmt_credits(counted)}</span>
                        <span class="text-[15px] font-medium text-zinc-500 tabular-nums">"/ "{fmt_credits(required)}</span>
                        <span class="flex-1"></span>
                        <span class=format!("{TAG} {TAG_ACCENT} tabular-nums")>{format!("{pct:.0}%")}</span>
                    </div>
                    <progress
                        class={if counted >= required { "progress ok" } else { "progress" }}
                        max=required
                        value=counted
                        aria-label={move || if is_thai.get() { "หน่วยกิตที่นับตามหลักสูตร" } else { "Credits counted toward the curriculum" }}
                        aria-valuetext={move || if is_thai.get() {
                            format!("{} จาก {} หน่วยกิต", fmt_credits(counted), fmt_credits(required))
                        } else {
                            format!("{} of {} credits", fmt_credits(counted), fmt_credits(required))
                        }}
                    ></progress>
                    <p class="mt-2 text-[12.5px] text-zinc-600">
                        {move || {
                            let base = if is_thai.get() {
                                format!("เรียนแล้วทั้งหมด {}", fmt_credits(earned))
                            } else {
                                format!("Earned in total {}", fmt_credits(earned))
                            };
                            if overflow > 0.0 {
                                if is_thai.get() {
                                    format!("{base} · เกินในเลือกเสรี {}", fmt_credits(overflow))
                                } else {
                                    format!("{base} · {} over in free electives", fmt_credits(overflow))
                                }
                            } else {
                                base
                            }
                        }}
                    </p>
                </div>

                <div class=format!("{CARD} px-4 py-4 sm:px-[18px]")>
                    <p class="text-[13px] text-zinc-600">{move || if is_thai.get() { "เกรดเฉลี่ยสะสม" } else { "Cumulative GPA" }}</p>
                    <p class="text-[28px] sm:text-[30px] leading-tight font-semibold tabular-nums mt-2 mb-2.5 text-zinc-900">
                        {gpa.map_or("—".to_string(), |g| format!("{g:.2}"))}
                    </p>
                    <p class="text-[12.5px] text-zinc-500 tabular-nums">
                        {move || {
                            let min = curriculum(&curriculum_id.get()).min_gpa;
                            if is_thai.get() { format!("ขั้นต่ำ {min:.2}") } else { format!("Minimum {min:.2}") }
                        }}
                    </p>
                </div>

                <div class=format!("{CARD} px-4 py-4 sm:px-[18px]")>
                    <p class="text-[13px] text-zinc-600">{move || if is_thai.get() { "ยังต้องเรียนอีก" } else { "Still to take" }}</p>
                    <p class="flex items-baseline gap-1.5 mt-2 mb-2.5">
                        <span class="text-[28px] sm:text-[30px] leading-tight font-semibold tabular-nums text-zinc-900">{fmt_credits(short)}</span>
                        <span class="text-[15px] font-medium text-zinc-500">{move || if is_thai.get() { "หน่วยกิต" } else { "credits" }}</span>
                    </p>
                    <p class="text-[12.5px] text-zinc-600">
                        {move || {
                            let th = is_thai.get();
                            if gaps.is_empty() {
                                if th { "ครบทุกหมวดแล้ว".to_string() } else { "Every category is complete".to_string() }
                            } else {
                                gaps.iter()
                                    .map(|(n, g)| format!("{} {}", label(n, th), fmt_credits(*g)))
                                    .collect::<Vec<_>>()
                                    .join(" · ")
                            }
                        }}
                    </p>
                </div>

                <div class=format!("{CARD} px-4 py-4 sm:px-[18px] col-span-2 min-[700px]:col-span-1")>
                    <p class="text-[13px] text-zinc-600">{move || if is_thai.get() { "ชั่วโมงเตรียมสหกิจ" } else { "Co-op prep hours" }}</p>
                    {move || {
                        let hours = faculty.get() + university.get();
                        if hours > 0.0 {
                            view! {
                                <p class="flex items-baseline gap-1.5 mt-2 mb-2.5">
                                    <span class="text-[28px] sm:text-[30px] leading-tight font-semibold tabular-nums text-zinc-900">{fmt_credits(hours)}</span>
                                    <span class="text-[15px] font-medium text-zinc-500 tabular-nums">"/ "{fmt_credits(TOTAL_REQUIRED)}</span>
                                    <span class="flex-1"></span>
                                    {(hours >= TOTAL_REQUIRED).then(|| view! { <span class=format!("{TAG} {TAG_OK}")>{move || if is_thai.get() { "ครบ" } else { "Done" }}</span> })}
                                </p>
                            }.into_any()
                        } else {
                            view! {
                                <p class="text-[17px] sm:text-[20px] font-semibold mt-2.5 mb-2.5 text-zinc-900">
                                    {move || if is_thai.get() { "ยังไม่ได้กรอก" } else { "Not entered yet" }}
                                </p>
                            }.into_any()
                        }
                    }}
                    <p class="text-[12.5px] text-zinc-600">
                        {move || if is_thai.get() { "กรอกได้ในแท็บภาพรวม" } else { "Enter them on the Overview tab" }}
                    </p>
                </div>
            </div>

            // ── Tabs ─────────────────────────────────
            <div
                class="flex gap-1 border-b border-zinc-200 mb-5 overflow-x-auto scrollbar-none print:hidden"
                role="tablist"
                aria-label={move || if is_thai.get() { "ส่วนของผลการตรวจ" } else { "Audit sections" }}
                on:keydown=on_tab_key
            >
                {Tab::ALL.iter().map(|&t| view! {
                    <button
                        id=format!("tab-{}", t.id())
                        role="tab"
                        type="button"
                        aria-controls=format!("panel-{}", t.id())
                        aria-selected=move || (tab.get() == t).to_string()
                        tabindex=move || if tab.get() == t { "0" } else { "-1" }
                        class=move || format!(
                            "relative shrink-0 px-3 pt-2.5 pb-3 text-[14.5px] font-medium whitespace-nowrap focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500 rounded-t-md {}",
                            if tab.get() == t {
                                "text-zinc-900 after:absolute after:left-2.5 after:right-2.5 after:-bottom-px after:h-0.5 after:rounded after:bg-zinc-900"
                            } else {
                                "text-zinc-500 hover:text-zinc-900"
                            }
                        )
                        on:click=move |_| tab.set(t)
                    >
                        {move || t.title(is_thai.get())}
                        {(t == Tab::Courses).then(|| view! {
                            <span class="ml-1.5 text-[11.5px] px-1.5 rounded-full border border-zinc-200 bg-zinc-50 text-zinc-500 tabular-nums">{course_count}</span>
                        })}
                    </button>
                }).collect::<Vec<_>>()}
            </div>

            {panel(Tab::Overview, view! { <Overview result=result.clone()/> }.into_any())}
            {panel(Tab::Courses, view! {
                <div class=format!("{CARD} overflow-hidden divide-y divide-zinc-200")>
                    {categories.iter().map(|category| {
                        let category = category.clone();
                        view! { <CategoryCard category={category} /> }
                    }).collect::<Vec<_>>()}
                </div>
            }.into_any())}
            {panel(Tab::Semesters, view! { <SemesterBreakdown result=result.clone()/> }.into_any())}
            {panel(Tab::Plan, view! { <WhatIfSimulator/> }.into_any())}
            {panel(Tab::Offerings, view! { <OfferingsPanel/> }.into_any())}

            <div class="mt-10 pt-5 border-t border-zinc-200 print:hidden">
                <SaveToggle/>
            </div>
        </div>
    }
}
