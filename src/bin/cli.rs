use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use mangad_neon::core::entities::inner::ExpireTime;
use mangad_neon::core::init::init_config;
use mangad_neon::core::repository::{IntoDatabaseUrl, Repository};
use mangad_neon::error::Error;
use mangad_neon::log;
use mangad_neon::core::config::LogConfig;

#[derive(Parser)]
#[command(name = "mangad-neon")]
#[command(about = "Manga Database and Crawler Management CLI", long_about = None)]
pub struct Cli {
    /// 数据库连接地址 (优先级: 参数 > 环境变量 > 配置文件)
    #[arg(
        long,
        env = "MANGAD_DATABASE_URL",
        global = true,
        help = "Database URL (e.g. postgres://user:pass@localhost:5432/db)"
    )]
    pub database_url: Option<String>,

    /// 日志级别 (优先级: 参数 > 环境变量 > 默认 error)
    #[arg(
        short = 'l',
        long = "log-level",
        env = "MANGAD_LOG_LEVEL",
        global = true,
        help = "Log level (trace, debug, info, warn, error)"
    )]
    pub log_level: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Token 相关操作 (生成、验证、吊销)
    Token(TokenArgs),
}

#[derive(Args)]
pub struct TokenArgs {
    #[command(subcommand)]
    pub action: TokenAction,
}

#[derive(Subcommand)]
pub enum TokenAction {
    /// 创建一个新的访问令牌
    Create {
        /// 过期时间: short (20m), medium (7d), long (30d), extended(90d), permanent(never)
        #[arg(short, long, default_value = "medium")]
        expire: String,

        /// 备注信息
        #[arg(short, long)]
        remark: Option<String>,

        /// 详细描述
        #[arg(short, long)]
        description: Option<String>,
    },
    /// 验证令牌有效性
    Verify {
        /// 要验证的 Token 字符串
        token: String,
    },
    /// 吊销指定令牌
    Revoke {
        /// 要吊销的 Token 字符串
        token: String,
    },
    /// 列出所有令牌
    List {
        /// 每页大小
        #[arg(default_value = "20")]
        size: u64,
        /// 页数
        #[arg(default_value = "0")]
        number: u64
    }
}

pub fn to_expire_time(s: &str) -> ExpireTime {
    match s {
        "short" => ExpireTime::Short,
        "medium" => ExpireTime::Medium,
        "long" => ExpireTime::Long,
        "extended" => ExpireTime::Extended,
        "permanent" => ExpireTime::Permanent,
        _ => ExpireTime::Long,
    }
}

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    // 初始化日志：命令行参数 > 环境变量 > 默认 error
    let log_level = cli.log_level.clone().unwrap_or_else(|| {
        std::env::var("MANGAD_LOG_LEVEL").unwrap_or_else(|_| "error".to_string())
    });

    log::init(&LogConfig {
        level: Some(log_level),
    });

    let config = init_config();

    let repo = match (config, cli.database_url) {
        (_, Some(database_url)) => Repository::new(&database_url).await?,
        (Ok(database_url), None) => Repository::new(&database_url.to_database_url()).await?,
        _ => {
            return Err(Error::CustomError("No Database URL provided".into()));
        }
    };

    match cli.command {
        Commands::Token(TokenArgs { action }) => match action {
            TokenAction::Create { expire, remark, description } => {
                let (token, secret) = repo
                    .create_token(to_expire_time(&expire), remark, description)
                    .await?;

                println!("Token created successfully!");
                println!("token: {}", secret);
                println!(
                    "expire time: {}",
                    match token.expire_time {
                        Some(time) => time.to_rfc2822(),
                        None => "Never".to_string()
                    }
                );
            }

            TokenAction::Verify { token } => {
                let ok = repo.verify_token(&token).await?;
                println!("Token verified: {}", ok);
            }

            TokenAction::Revoke { token } => {
                match repo.revoke_token(&token).await {
                    Ok(_) => println!("Token successfully revoked"),
                    Err(e) => println!("Revocation failed {e}"),
                }
            }

            TokenAction::List { size, number } => {
                let tokens = repo.list_tokens(size, number).await?;
                println!("Now: {}", Utc::now().to_rfc2822());
                println!(
                    "+-{:36}-+-{:10}-+-{:15}-+-{:31}-+-{:31}-+-{:9}-+",
                    "-".repeat(36),
                    "-".repeat(10),
                    "-".repeat(15),
                    "-".repeat(31),
                    "-".repeat(31),
                    "-".repeat(9),
                );
                println!(
                    "| {:36} | {:10} | {:15} | {:31} | {:31} | {:9} |",
                    "UUID",
                    "REMARK",
                    "DESCRIPTION",
                    "CREATE AT",
                    "EXPIRE AT",
                    "STATUS"
                );
                println!(
                    "+-{:36}-+-{:10}-+-{:15}-+-{:31}-+-{:31}-+-{:9}-+",
                    "-".repeat(36),
                    "-".repeat(10),
                    "-".repeat(15),
                    "-".repeat(31),
                    "-".repeat(31),
                    "-".repeat(9),
                );
                for token in tokens {
                    println!(
                        "| {:36} | {:10} | {:15} | {:31} | {:31} | {:9} |",
                        token.id,
                        token.remark.unwrap_or_else(|| "None".to_string()),
                        token.description.unwrap_or_else(|| "None".to_string()),
                        token.create_time.to_rfc2822(),
                        if let Some(t) = token.expire_time {
                            t.to_rfc2822()
                        } else {
                            "Never".to_string()
                        },
                        match (token.expire_time, token.is_revoked) {
                            (_, true) => "Revoked",
                            (Some(t), false) => {
                                if Utc::now() > t {
                                    "Expired"
                                } else {
                                    "Available"
                                }
                            }
                            (None, false) => "Available",
                        },
                    );
                }
                println!(
                    "+-{:36}-+-{:10}-+-{:15}-+-{:31}-+-{:31}-+-{:9}-+",
                    "-".repeat(36),
                    "-".repeat(10),
                    "-".repeat(15),
                    "-".repeat(31),
                    "-".repeat(31),
                    "-".repeat(9),
                );
                println!("Page Size: {}\nPage Number: {}", size, number);
            }
        },
    }

    Ok(())
}
