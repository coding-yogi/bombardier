use flexi_logger::{Duplicate, FileSpec, Logger, opt_format};

pub fn initiate(with_file: bool) {
    let mut logger = Logger::try_with_env().unwrap().format(opt_format);
            
    if with_file {
        logger = logger.log_to_file(FileSpec::default()).duplicate_to_stderr(Duplicate::Info).print_message();
    }

    logger.start().unwrap();
}