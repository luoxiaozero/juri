use crate::{
    byte::{handle_bytes, send_stream},
    cache::main::init_cache,
    plugin::JuriPlugin,
    routing::{conversion_router, match_route, MatchRouter, Router},
    Config, JuriError, Request, Response,
};
use async_std::{
    net::{TcpListener, TcpStream},
    prelude::*,
    sync::Arc,
};
use colored::*;
use std::{collections::HashMap, net::SocketAddr};

pub struct Server {
    addr: SocketAddr,
    plugins: Vec<Box<dyn JuriPlugin>>,
    config: Config,
}

impl Server {
    pub fn bind(addr: SocketAddr) -> Self {
        Server {
            addr,
            plugins: vec![],
            config: Config::new(),
        }
    }

    pub fn config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    pub fn plugin(mut self, plugin: impl JuriPlugin) -> Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    pub async fn server(
        self,
        router: Router,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        init_cache();
        let listener = TcpListener::bind(self.addr).await?;
        println!(
            "{}: listener port http://{} start",
            "Juri".green(),
            self.addr
        );
        let mut incoming = listener.incoming();
        let router = Arc::new(conversion_router(router));
        let plugins = Arc::new(self.plugins);
        let config = Arc::new(self.config);

        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            let router = Arc::clone(&router);
            let plugins = Arc::clone(&plugins);
            let config = Arc::clone(&config);

            Server::handle_stream(stream, router, plugins, config).await;
        }
        Ok(())
    }

    async fn handle_stream(
        mut stream: TcpStream,
        router: Arc<MatchRouter>,
        plugins: Arc<Vec<Box<dyn JuriPlugin>>>,
        config: Arc<Config>,
    ) {
        loop {
            let router = Arc::clone(&router);
            let plugins = Arc::clone(&plugins);
            let config = Arc::clone(&config);

            match handle_bytes(&mut stream, &config).await {
                Ok(mut request) => {
                    let peer_addr = stream.peer_addr().unwrap().ip();
                    println!(
                        "{}: Request {} {} {}",
                        "INFO".green(),
                        request.method,
                        request.path,
                        peer_addr
                    );

                    let mut plugin = plugins.iter();
                    let plugin_response = loop {
                        match plugin.next() {
                            Some(plugin) => {
                                let response = plugin.request(&mut request);
                                if let Some(response) = response {
                                    break Some(response);
                                }
                            }
                            None => break None,
                        }
                    };

                    let mut response = match plugin_response {
                        Some(response) => response,
                        None => match match_route(&mut request, router) {
                            Some(fun) => {
                                let response: crate::Result<Response> = fun(&request);
                                match response {
                                    Ok(response) => response,
                                    Err(err) => match err {
                                        JuriError::CustomError(_) => (response_500)(&request),
                                        JuriError::ResponseError(response) => response,
                                    },
                                }
                            }
                            None => (response_404)(&request),
                        },
                    };

                    for plugin in plugins.iter() {
                        plugin.response(&request, &mut response);
                    }

                    println!(
                        "{}: Response {} {} {}",
                        "INFO".green(),
                        request.method,
                        request.path,
                        response.status_code
                    );

                    send_stream(&mut stream, &config, Some(&request), &response).await;

                    if !request.is_keep_alive() {
                        break;
                    }
                }
                Err(e) => {
                    match e.code {
                        405 => {
                            let response = Response {
                                status_code: 405,
                                contents: "".to_string(),
                                headers: HashMap::new(),
                            };
                            send_stream(&mut stream, &config, None, &response).await;
                        }
                        500 => {
                            let response = Response {
                                status_code: 500,
                                contents: "<h1>500</h1>".to_string(),
                                headers: HashMap::from([(
                                    "Content-Type".to_string(),
                                    "text/html;charset=utf-8".to_string(),
                                )]),
                            };
                            send_stream(&mut stream, &config, None, &response).await;
                        }
                        _ => {
                            println!("{}: {:?}", "ERROR".red(), e);
                        }
                    }
                    break;
                }
            };
        }
    }
}

fn response_404(_request: &Request) -> Response {
    Response {
        status_code: 404,
        contents: "<h1>404</h1>".to_string(),
        headers: HashMap::from([(
            "Content-Type".to_string(),
            "text/html;charset=utf-8".to_string(),
        )]),
    }
}

fn response_500(_request: &Request) -> Response {
    Response {
        status_code: 500,
        contents: "<h1>500</h1>".to_string(),
        headers: HashMap::from([(
            "Content-Type".to_string(),
            "text/html;charset=utf-8".to_string(),
        )]),
    }
}
