//! Overview tab: progress by category (with the major's groups and any
//! overflow), what to do next, GPA by semester, and the co-op hours card.

use crate::app::AppState;
use crate::components::coop_hours::CoopHours;
use crate::components::offerings::fetch_json;
use crate::components::results::{SelectedTab, Tab};
use crate::components::status::{by_semester, fmt_credits, label, todo_rows, Todo};
use crate::components::ui::{CARD, SECTION_TITLE, TAG, TAG_MUTED, TAG_OK, TAG_WARN};
use audit_core::models::{compute_gpa, AuditResult};
use audit_core::offerings::{OfferingsIndex, TermOfferings};
use leptos::prelude::*;

/// Newest term in the offerings index and the course codes it lists.
async fn newest_term() -> Option<(String, Vec<String>)> {
    let index = fetch_json::<OfferingsIndex>("offerings/index.json".into()).await?;
    let key = index.terms.first()?.clone();
    let term = fetch_json::<TermOfferings>(format!("offerings/{key}.json")).await?;
    Some((
        term.term,
        term.courses.into_iter().map(|c| c.code).collect(),
    ))
}

#[component]
pub fn Overview(result: AuditResult) -> impl IntoView {
    let AppState {
        audit_result,
        is_thai,
        ..
    } = expect_context::<AppState>();
    let SelectedTab(tab) = expect_context::<SelectedTab>();
    let offered = LocalResource::new(newest_term);

    let copy_missing = move |_| {
        let Some(result) = audit_result.get_untracked() else {
            return;
        };
        let out: String = result
            .missing_subjects
            .iter()
            .map(|m| format!("[{}] {}\n", m.category, m.description))
            .collect();
        if let Some(clipboard) = web_sys::window().map(|w| w.navigator().clipboard()) {
            let _ = clipboard.write_text(&out);
        }
    };

    let todos = todo_rows(&result.missing_subjects);
    let todo_count = todos.len();
    let categories = result.categories.clone();

    // Semester GPA bars, oldest first; only with at least two graded terms.
    let gpa_bars: Vec<(String, f32)> = by_semester(&result)
        .into_iter()
        .filter_map(|(sem, courses)| compute_gpa(&courses).map(|g| (sem, g)))
        .collect();

    let tag = |kind: &str| format!("{TAG} {kind}");

    view! {
        <div class="grid gap-4 lg:grid-cols-[minmax(0,1.55fr)_minmax(0,1fr)] items-start">
            // ── Progress by category ────────────────
            <section class=format!("{CARD} overflow-hidden")>
                <div class="flex items-center gap-2.5 px-4 sm:px-[18px] pt-4 pb-1.5">
                    <h2 class=SECTION_TITLE>{move || if is_thai.get() { "ความคืบหน้าตามหมวด" } else { "Progress by category" }}</h2>
                    <span class="flex-1"></span>
                    <span class="text-[12.5px] text-zinc-400">{move || if is_thai.get() { "หน่วยกิต" } else { "credits" }}</span>
                </div>
                {categories.into_iter().map(|c| {
                    let name = c.name.clone();
                    let complete = c.is_complete();
                    let overflow = c.overflow_credits();
                    let gap = c.required_credits - c.counted_credits();
                    let (collected, required) = (c.collected_credits, c.required_credits);
                    let counted_share = if collected > 0.0 { (required / collected * 100.0).min(100.0) } else { 0.0 };
                    let groups = c.groups.clone();
                    view! {
                        <div class="grid grid-cols-[minmax(0,1fr)_auto] gap-x-3.5 gap-y-1.5 items-center px-4 sm:px-[18px] py-3 border-t border-zinc-200">
                            <div class="flex items-center gap-2 min-w-0 font-medium text-[14.5px] text-zinc-900">
                                <span class="truncate">{move || label(&name, is_thai.get())}</span>
                                {if overflow > 0.0 {
                                    view! { <span class=tag(TAG_MUTED)>{move || if is_thai.get() { format!("เกิน +{}", fmt_credits(overflow)) } else { format!("+{} over", fmt_credits(overflow)) }}</span> }.into_any()
                                } else if complete {
                                    view! { <span class=tag(TAG_OK)>{move || if is_thai.get() { "ครบ" } else { "Done" }}</span> }.into_any()
                                } else {
                                    view! { <span class=tag(TAG_WARN)>{move || if is_thai.get() { format!("ขาด {}", fmt_credits(gap)) } else { format!("{} short", fmt_credits(gap)) }}</span> }.into_any()
                                }}
                            </div>
                            <div class="text-[13.5px] text-zinc-500 text-right tabular-nums">
                                <b class="font-semibold text-zinc-900">{fmt_credits(collected)}</b>" / "{fmt_credits(required)}
                            </div>
                            <div class="col-span-2">
                                {if overflow > 0.0 {
                                    view! {
                                        <div class="flex h-2 rounded-full overflow-hidden bg-zinc-200" role="img"
                                            aria-label={move || if is_thai.get() {
                                                format!("นับได้ {} หน่วยกิต เกิน {} หน่วยกิต", fmt_credits(required), fmt_credits(overflow))
                                            } else {
                                                format!("{} credits counted, {} over", fmt_credits(required), fmt_credits(overflow))
                                            }}>
                                            <span class="block h-full bg-emerald-700" style=format!("width:{counted_share:.1}%")></span>
                                            <span class="block h-full flex-1 hatch"></span>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <progress class={if complete { "progress ok" } else { "progress" }} max=required value=collected
                                            aria-valuetext={move || if is_thai.get() { format!("{} จาก {}", fmt_credits(collected), fmt_credits(required)) } else { format!("{} of {}", fmt_credits(collected), fmt_credits(required)) }}></progress>
                                    }.into_any()
                                }}
                            </div>
                        </div>
                        {groups.into_iter().map(|g| {
                            let gname = g.name.clone();
                            let done = g.collected_credits >= g.required_credits;
                            view! {
                                <div class="grid grid-cols-[minmax(0,1fr)_auto] gap-x-3.5 gap-y-1.5 items-center pl-7 sm:pl-[38px] pr-4 sm:pr-[18px] py-2.5 border-t border-zinc-200 bg-zinc-50/70">
                                    <div class="text-sm text-zinc-800 truncate">{move || label(&gname, is_thai.get())}</div>
                                    <div class="text-[13.5px] text-zinc-500 text-right tabular-nums">
                                        <b class="font-semibold text-zinc-900">{fmt_credits(g.collected_credits)}</b>" / "{fmt_credits(g.required_credits)}
                                    </div>
                                    <progress class={if done { "progress thin ok col-span-2" } else { "progress thin col-span-2" }} max=g.required_credits value=g.collected_credits></progress>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    }
                }).collect::<Vec<_>>()}
                <div class="flex flex-wrap gap-x-3.5 gap-y-1 px-4 sm:px-[18px] pt-2.5 pb-4 border-t border-zinc-200 text-[12.5px] text-zinc-600">
                    <span class="inline-flex items-center gap-1.5"><i class="inline-block w-2.5 h-2.5 rounded-sm bg-emerald-700"></i>{move || if is_thai.get() { "ครบแล้ว" } else { "Complete" }}</span>
                    <span class="inline-flex items-center gap-1.5"><i class="inline-block w-2.5 h-2.5 rounded-sm bg-accent-600"></i>{move || if is_thai.get() { "กำลังสะสม" } else { "In progress" }}</span>
                    <span class="inline-flex items-center gap-1.5"><i class="inline-block w-2.5 h-2.5 rounded-sm hatch"></i>{move || if is_thai.get() { "เกินเกณฑ์ ไม่นับเพิ่ม" } else { "Over the requirement, not counted" }}</span>
                </div>
            </section>

            <div class="grid gap-4 content-start">
                // ── What to do next ─────────────────────
                <section class=format!("{CARD} overflow-hidden")>
                    <div class="flex items-center gap-2.5 px-4 sm:px-[18px] pt-4 pb-1.5">
                        <h2 class=SECTION_TITLE>{move || if is_thai.get() { "สิ่งที่ต้องทำต่อ" } else { "What to do next" }}</h2>
                        <span class="flex-1"></span>
                        {(todo_count > 0).then(|| view! {
                            <span class=format!("{TAG} {TAG_WARN} tabular-nums")>{move || if is_thai.get() { format!("{todo_count} รายการ") } else { format!("{todo_count} items") }}</span>
                            <button type="button" class="text-xs font-medium text-zinc-500 hover:text-zinc-900 rounded px-1.5 py-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500" on:click=copy_missing>
                                {move || if is_thai.get() { "คัดลอก" } else { "Copy" }}
                            </button>
                        })}
                    </div>
                    {if todos.is_empty() {
                        view! {
                            <p class="px-4 sm:px-[18px] pt-2 pb-4 text-sm text-zinc-600 border-t border-zinc-200">
                                {move || if is_thai.get() { "ไม่มีรายการค้าง" } else { "Nothing left" }}
                            </p>
                        }.into_any()
                    } else {
                        todos.into_iter().map(|t| {
                            let (code, title, sub, credits) = match t.clone() {
                                Todo::Course { code, name, category, credits } => (Some(code), name, Some(category), Some(credits)),
                                Todo::Gap { category, credits } => (None, category.clone(), None, Some(credits)),
                                Todo::Rule { category, description } => (None, description, Some(category), None),
                            };
                            let is_gap = matches!(t, Todo::Gap { .. });
                            let code_for_offer = code.clone();
                            view! {
                                <div class="flex gap-3 px-4 sm:px-[18px] py-3 border-t border-zinc-200 items-start">
                                    <div class="w-[30px] h-[30px] rounded-lg grid place-items-center shrink-0 bg-amber-50 text-amber-700">
                                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24">
                                            {if code.is_some() { view! { <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20V3H6.5A2.5 2.5 0 0 0 4 5.5z"/> }.into_any() } else { view! { <path d="M4 6h16M4 12h16M4 18h10"/> }.into_any() }}
                                        </svg>
                                    </div>
                                    <div class="min-w-0 flex-1">
                                        <p class="font-medium text-[14.5px] leading-normal text-zinc-900">
                                            {move || if is_gap {
                                                if is_thai.get() { format!("{} อีก", label(&title, true)) } else { format!("More {}", label(&title, false).to_lowercase()) }
                                            } else {
                                                title.clone()
                                            }}
                                            {code.clone().map(|c| view! { " "<span class="font-mono text-[0.86em] text-zinc-400">{c}</span> })}
                                        </p>
                                        {sub.map(|s| view! { <p class="text-[13px] text-zinc-500">{move || label(&s, is_thai.get())}</p> })}
                                        {move || {
                                            let code = code_for_offer.clone()?;
                                            let (term, codes) = offered.get().flatten()?;
                                            codes.contains(&code).then(|| view! {
                                                <button type="button" class="text-[13px] font-medium text-accent-700 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500 rounded" on:click=move |_| tab.set(Tab::Offerings)>
                                                    {if is_thai.get() { format!("เปิดสอน {term} →") } else { format!("Offered {term} →") }}
                                                </button>
                                            })
                                        }}
                                    </div>
                                    {credits.filter(|c| *c > 0.0).map(|c| view! {
                                        <span class="ml-auto text-[13px] text-zinc-500 whitespace-nowrap tabular-nums">
                                            {move || if is_thai.get() { format!("{} นก.", fmt_credits(c)) } else { format!("{} cr", fmt_credits(c)) }}
                                        </span>
                                    })}
                                </div>
                            }
                        }).collect::<Vec<_>>().into_any()
                    }}
                </section>

                // ── GPA by semester ─────────────────────
                {(gpa_bars.len() >= 2).then(|| {
                    let last = gpa_bars.len() - 1;
                    let summary = gpa_bars.iter().map(|(s, g)| format!("{s} {g:.2}")).collect::<Vec<_>>().join(", ");
                    view! {
                        <section class=CARD>
                            <div class="flex items-center gap-2.5 px-4 sm:px-[18px] pt-4 pb-1.5">
                                <h2 class=SECTION_TITLE>{move || if is_thai.get() { "เกรดเฉลี่ยรายเทอม" } else { "GPA by semester" }}</h2>
                                <span class="flex-1"></span>
                                <span class="text-[12.5px] text-zinc-400 tabular-nums">{move || if is_thai.get() { "สูงสุด 4.00" } else { "Max 4.00" }}</span>
                            </div>
                            <div class="flex items-end gap-2.5 h-[150px] px-4 sm:px-[18px] pt-2" role="img" aria-label=summary>
                                {gpa_bars.iter().enumerate().map(|(i, (_, g))| view! {
                                    <div class="flex-1 min-w-0 h-full flex flex-col items-center justify-end gap-1.5">
                                        <span class="text-xs text-zinc-500 tabular-nums">{format!("{g:.2}")}</span>
                                        <div
                                            class=if i == last { "w-full max-w-[34px] rounded-t-md rounded-b-sm bg-accent-600" } else { "w-full max-w-[34px] rounded-t-md rounded-b-sm bg-accent-100 border border-accent-300" }
                                            style=format!("height:{:.0}%", g / 4.0 * 100.0)
                                        ></div>
                                    </div>
                                }).collect::<Vec<_>>()}
                            </div>
                            <div class="flex gap-2.5 px-4 sm:px-[18px] pt-1.5 pb-4">
                                {gpa_bars.iter().map(|(s, _)| {
                                    let short = s.replace("/25", "/");
                                    view! { <span class="flex-1 min-w-0 text-center text-[11.5px] text-zinc-400 whitespace-nowrap">{short}</span> }
                                }).collect::<Vec<_>>()}
                            </div>
                        </section>
                    }
                })}

                <CoopHours/>
            </div>
        </div>
    }
}
