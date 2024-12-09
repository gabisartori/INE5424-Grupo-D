/// creates a folder for each tests, receives the number of tests as an argument
#[macro_export]
macro_rules! initializate_folders {
    ($tests_num:expr) => {
        use std::fs;
        // deletes the needed folder if they exists
        if (fs::metadata("tests").is_ok()) {
            fs::remove_dir_all("tests").expect("Erro ao deletar a pasta 'tests'");
        };
        fs::create_dir_all("tests").expect("Erro ao criar a pasta 'tests'");

        if (fs::metadata("src/log").is_ok()) {
            fs::remove_dir_all("src/log").expect("Erro ao deletar a pasta 'src/log'");
        };
        fs::create_dir_all("src/log").expect("Erro ao criar a pasta 'src/log'");

        if (fs::metadata("relcomm/log").is_ok()) {
            fs::remove_dir_all("relcomm/log").expect("Erro ao deletar a pasta 'relcomm/log'");
        };
        fs::create_dir_all("relcomm/log").expect("Erro ao criar a pasta 'relcomm/log'");

        // creates a folder for each test
        for i in 0..$tests_num {
            let path = format!("tests/test_{}", i);
            let error_msg = format!("Erro ao criar a pasta '{}'", path);
            fs::create_dir_all(path.clone()).expect(&error_msg);
            // creates a result file for each test
            let path = format!("{}/debug_agts", path);
            fs::create_dir_all(path.clone()).expect(&error_msg);
            // File::create(path).expect("Erro ao criar o arquivo de resultado");

            let path = format!("src/log/test_{}", i);
            let error_msg = format!("Erro ao criar a pasta '{}'", path);
            fs::create_dir_all(path.clone()).expect(&error_msg);
        }
    };
}

/// writes a message in a file
#[macro_export]
macro_rules! debug_file {
    ($file_path:expr, $msg:expr) => {
        let mut file: std::fs::File = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open($file_path)
        {
            Ok(f) => f,
            Err(e) => panic!("Erro ao abrir o arquivo: {}", e),
        };
        // connverts the message to a string
        // Implements Logger for creating a debug log file
        std::io::Write::write_all(&mut file, $msg).expect("Erro ao escrever no arquivo");
    };
}

/// writes a message in tests/debug.txt
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        let args = std::env::args().collect::<Vec<String>>();
        let (path0, path1, id) = if args.len() > 2 {
            (format!("tests/test_{}/debug.txt", args[1]),
            format!("tests/test_{}/debug_agts/debug_agt_{}.txt", args[1], args[2]),
            args[2].clone())
        } else {
            (format!("tests/debug.txt"), 
            format!("tests/debug_main.txt"),
            "main".to_string())
        };
        /// writes a message in a all-purpose debug file
        let mut file: std::fs::File = match std::fs::OpenOptions::new()
                                            .create(true)
                                            .append(true)
                                            .open(path0) {
            Ok(f) => f,
            Err(e) => panic!("Erro ao abrir o arquivo: {}", e)
        };
        let msf = format!("----------\nAgente {id}\n{}\n----------\n", format!($($arg)*));
        std::io::Write::write_all(&mut file, msf.as_bytes()).expect("Erro ao escrever no arquivo");
        /// writes a message in a debug file for each agent
        let mut file: std::fs::File = match std::fs::OpenOptions::new()
                                            .create(true)
                                            .append(true)
                                            .open(path1) {
            Ok(f) => f,
            Err(e) => panic!("Erro ao abrir o arquivo: {}", e)
        };
        let msf = format!("----------\n{}\n----------\n", format!($($arg)*));
        std::io::Write::write_all(&mut file, msf.as_bytes()).expect("Erro ao escrever no arquivo");
    };
}
