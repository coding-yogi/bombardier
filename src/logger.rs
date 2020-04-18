use flexi_logger::{Duplicate, Logger, opt_format};

pub fn initiate(with_file: bool) {
    let mut logger = Logger::with_env().format(opt_format);
            
    if with_file {
        logger = logger.log_to_file().duplicate_to_stderr(Duplicate::Warn).print_message();
    }

    logger.start().unwrap();
}