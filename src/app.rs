//! Top-level `App` Leptos component.
//!
//! Owns the shared UI state (upload, audit result, parse mode, persistence,
//! What-If plan), provides it to `components::*` as `AppState`, and
//! orchestrates reactive re-audits via `audit_core::auditor::build_audit`.
//! Section markup and handlers live in `src/components/`.

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Title};

use crate::components::coop_hours::provide_coop_hours;
use crate::components::empty_state::EmptyState;
use crate::components::header::Header;
use crate::components::offerings::OfferingsPanel;
use crate::components::results::{ResultsView, SelectedTab, Tab};
use audit_core::auditor::build_audit;
use audit_core::data::{curricula, curriculum};
use audit_core::models::{AuditResult, ParseMode, ParsedCourse};

/// Signals shared by the UI components, provided once by `App` via context.
#[derive(Clone, Copy)]
pub struct AppState {
    pub file_name: ReadSignal<String>,
    pub set_file_name: WriteSignal<String>,
    pub preview_url: ReadSignal<Option<String>>,
    pub set_preview_url: WriteSignal<Option<String>>,
    pub audit_result: ReadSignal<Option<AuditResult>>,
    pub set_audit_result: WriteSignal<Option<AuditResult>>,
    /// Transcript plus What-If planned courses; `None` when nothing is planned.
    /// `audit_result` always reflects the transcript alone.
    pub plan_result: ReadSignal<Option<AuditResult>>,
    pub is_loading: ReadSignal<bool>,
    pub set_is_loading: WriteSignal<bool>,
    pub error_msg: ReadSignal<Option<String>>,
    pub set_error_msg: WriteSignal<Option<String>>,
    pub dropped_file: ReadSignal<Option<web_sys::File>, LocalStorage>,
    pub set_dropped_file: WriteSignal<Option<web_sys::File>, LocalStorage>,
    pub parse_mode: ReadSignal<ParseMode>,
    pub set_parse_mode: WriteSignal<ParseMode>,
    pub parsed_courses: ReadSignal<Vec<ParsedCourse>>,
    pub set_parsed_courses: WriteSignal<Vec<ParsedCourse>>,
    pub planned_courses: ReadSignal<Vec<ParsedCourse>>,
    pub set_planned_courses: WriteSignal<Vec<ParsedCourse>>,
    pub plan_code: ReadSignal<String>,
    pub set_plan_code: WriteSignal<String>,
    pub plan_name: ReadSignal<String>,
    pub set_plan_name: WriteSignal<String>,
    pub plan_credit: ReadSignal<f32>,
    pub set_plan_credit: WriteSignal<f32>,
    pub plan_grade: ReadSignal<String>,
    pub set_plan_grade: WriteSignal<String>,
    pub curriculum_id: ReadSignal<String>,
    /// Admission cohort from the student ID on the analysed PDF (e.g. 2567).
    pub cohort: ReadSignal<Option<u32>>,
    pub set_cohort: WriteSignal<Option<u32>>,
    pub set_curriculum_id: WriteSignal<String>,
    pub save_enabled: ReadSignal<bool>,
    pub set_save_enabled: WriteSignal<bool>,
    pub is_thai: ReadSignal<bool>,
    pub set_is_thai: WriteSignal<bool>,
}

/// Main application component
///
/// Manages state for file upload, PDF preview, audit results, and loading state.
/// Implements drag-and-drop PDF upload with real-time PDF preview.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    // State management for file upload and audit processing
    let (file_name, set_file_name) = signal(String::new());
    let (preview_url, set_preview_url) = signal(Option::<String>::None);
    let (audit_result, set_audit_result) = signal(Option::<AuditResult>::None);
    let (plan_result, set_plan_result) = signal(Option::<AuditResult>::None);
    let (is_loading, set_is_loading) = signal(false);
    let (error_msg, set_error_msg) = signal(Option::<String>::None);
    // Stores the file from drag-and-drop (file input is not updated by drop events)
    let (dropped_file, set_dropped_file) = signal_local(Option::<web_sys::File>::None);
    // Which PDF format was detected during parsing (drives the mode banner).
    let (parse_mode, set_parse_mode) = signal(ParseMode::Grade);
    // Raw parsed courses from the PDF (kept around so the What-If simulator
    // can re-run audits without re-parsing).
    let (parsed_courses, set_parsed_courses) = signal(Vec::<ParsedCourse>::new());
    // User-added hypothetical courses appended to the audit input.
    let (planned_courses, set_planned_courses) = signal(Vec::<ParsedCourse>::new());
    // Form fields for the simulator
    let (plan_code, set_plan_code) = signal(String::new());
    let (plan_name, set_plan_name) = signal(String::new());
    let (plan_credit, set_plan_credit) = signal(3.0_f32);
    let (plan_grade, set_plan_grade) = signal("A".to_string());

    // Persistence — opt-in localStorage of the last audit result (key "audit_v1").
    // Remains off by default to stay privacy-first.
    let (save_enabled, set_save_enabled) = signal(false);
    // Curriculum revision the audit runs against (see audit_core::data).
    let (curriculum_id, set_curriculum_id) = signal(curricula()[0].id.clone());
    let (cohort, set_cohort) = signal(None::<u32>);

    // Try to rehydrate both the toggle and a prior result on mount.
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(Some(flag)) = storage.get_item("audit_save_enabled") {
            if flag == "1" {
                set_save_enabled.set(true);
                if let Ok(Some(raw)) = storage.get_item("audit_v1") {
                    if let Ok(saved) = serde_json::from_str::<AuditResult>(&raw) {
                        set_audit_result.set(Some(saved));
                    }
                }
                // The transcript itself, so What-If and curriculum changes re-audit it
                // (older saves only have the result above).
                if let Ok(Some(raw)) = storage.get_item("audit_parsed_v1") {
                    if let Ok(saved) = serde_json::from_str::<Vec<ParsedCourse>>(&raw) {
                        set_parsed_courses.set(saved);
                    }
                }
                if let Ok(Some(id)) = storage.get_item("audit_curriculum_v1") {
                    set_curriculum_id.set(id);
                }
                if let Ok(Some(c)) = storage.get_item("audit_cohort_v1") {
                    set_cohort.set(c.parse().ok());
                }
                if let Ok(Some(mode_raw)) = storage.get_item("audit_mode_v1") {
                    if mode_raw == "registration" {
                        set_parse_mode.set(ParseMode::Registration);
                    }
                }
            }
        }
    }

    // Re-run the audit whenever the parsed transcript or the curriculum
    // changes. Skip when nothing has been parsed yet so the empty-state keeps
    // rendering. Planned courses are not read here, so planning never rebuilds
    // the results view.
    Effect::new(move |_| {
        let parsed = parsed_courses.get();
        if parsed.is_empty() {
            return;
        }
        let cur_id = curriculum_id.get();
        if save_enabled.get_untracked() {
            if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten())
            {
                if let Ok(json) = serde_json::to_string(&parsed) {
                    let _ = storage.set_item("audit_parsed_v1", &json);
                }
                let _ = storage.set_item("audit_curriculum_v1", &cur_id);
                if let Some(c) = cohort.get_untracked() {
                    let _ = storage.set_item("audit_cohort_v1", &c.to_string());
                }
            }
        }
        let result = build_audit(&parsed, curriculum(&cur_id));
        if save_enabled.get_untracked() {
            if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten())
            {
                if let Ok(json) = serde_json::to_string(&result) {
                    let _ = storage.set_item("audit_v1", &json);
                }
            }
        }
        set_audit_result.set(Some(result));
    });

    // Transcript plus What-If courses, for the Plan tab's projection only.
    Effect::new(move |_| {
        let planned = planned_courses.get();
        let parsed = parsed_courses.get();
        let cur_id = curriculum_id.get();
        set_plan_result.set((!planned.is_empty() && !parsed.is_empty()).then(|| {
            let mut combined = parsed;
            combined.extend(planned);
            build_audit(&combined, curriculum(&cur_id))
        }));
    });

    provide_coop_hours(save_enabled);
    provide_context(SelectedTab(RwSignal::new(Tab::Overview)));

    // Language toggle: true = Thai (default), false = English
    let (is_thai, set_is_thai) = signal(true);
    provide_context(is_thai);
    provide_context(AppState {
        file_name,
        set_file_name,
        preview_url,
        set_preview_url,
        audit_result,
        set_audit_result,
        plan_result,
        is_loading,
        set_is_loading,
        error_msg,
        set_error_msg,
        dropped_file,
        set_dropped_file,
        parse_mode,
        set_parse_mode,
        parsed_courses,
        set_parsed_courses,
        planned_courses,
        set_planned_courses,
        plan_code,
        set_plan_code,
        plan_name,
        set_plan_name,
        plan_credit,
        set_plan_credit,
        plan_grade,
        set_plan_grade,
        curriculum_id,
        set_curriculum_id,
        cohort,
        set_cohort,
        save_enabled,
        set_save_enabled,
        is_thai,
        set_is_thai,
    });

    // Switch views only when a result appears or goes away, so re-setting
    // `audit_result` to None does not rebuild (and dispose) the empty state.
    let has_result = Memo::new(move |_| audit_result.with(Option::is_some));

    view! {
        <Title text="Course Audit — PSU CS"/>

        <div class="min-h-screen font-sans bg-zinc-50 text-zinc-900 selection:bg-accent-100">
            <Header/>
            <main class="max-w-[1200px] w-full mx-auto px-3.5 sm:px-5 pt-5 sm:pt-7 pb-20" aria-live="polite" aria-atomic="false">
                {move || if has_result.get() {
                    view! {
                        {move || audit_result.get().map(|result| view! { <ResultsView result=result/> })}
                    }.into_any()
                } else {
                    // Offerings stay searchable without a transcript.
                    view! {
                        <EmptyState/>
                        <div class="mt-12">
                            <OfferingsPanel/>
                        </div>
                    }.into_any()
                }}
            </main>
        </div>
    }
}
