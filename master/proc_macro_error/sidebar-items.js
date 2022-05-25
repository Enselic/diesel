initSidebarItems({"attr":[["proc_macro_error",""]],"enum":[["Level","Represents a diagnostic level"]],"fn":[["abort_if_dirty","Abort macro execution and display all the emitted errors, if any."]],"macro":[["abort","Abort proc-macro execution right now and display the error."],["abort_call_site","Shortcut for `abort!(Span::call_site(), msg...)`. This macro is still preferable over plain panic, panics are not for error reporting."],["diagnostic","Build `Diagnostic` instance from provided arguments."],["emit_call_site_error","Shortcut for `emit_error!(Span::call_site(), ...)`. This macro is still preferable over plain panic, panics are not for error reporting.."],["emit_call_site_warning","Shortcut for `emit_warning!(Span::call_site(), ...)`."],["emit_error","Emit an error while not aborting the proc-macro right away."],["emit_warning","Emit a warning. Warnings are not errors and compilation won’t fail because of them."]],"mod":[["dummy","Facility to emit dummy implementations (or whatever) in case an error happen."]],"struct":[["Diagnostic","Represents a single diagnostic message"],["SpanRange",""]],"trait":[["DiagnosticExt","A collection of methods that do not exist in `proc_macro::Diagnostic` but still useful to have around."],["OptionExt","This traits expands `Option` with some handy shortcuts."],["ResultExt","This traits expands `Result<T, Into<Diagnostic>>` with some handy shortcuts."]]});