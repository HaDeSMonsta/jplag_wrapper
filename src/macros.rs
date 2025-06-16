macro_rules! handle_sub_err {
    ($err_msg:expr, $to_execute:expr, $errs:expr, $abort_on_err:expr) => {
        let _ = $to_execute;
        let err = anyhow::anyhow!($err_msg);
        if $abort_on_err {
            return core::result::Result::Err(err);
        }
        $errs.push(err);
    };
}
