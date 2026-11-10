use anyhow::Result;
use std::process::Command;

/// Vite removed --https. Preserve the application's Vite configuration and let
/// its certificate settings or SSL plugin provide TLS, then verify the result.
pub(super) fn configure_https(
    command: &mut Command,
    args: &mut Vec<String>,
    https: bool,
) -> Result<()> {
    if !https {
        return Ok(());
    }
    std::fs::write(
        "platforms/web/.jffi-https.config.mjs",
        r#"import { loadConfigFromFile, mergeConfig } from 'vite';
export default async (env) => {
  const original = await loadConfigFromFile(env);
  return mergeConfig(original?.config ?? {}, {
    plugins: [{
      name: 'jffi-require-https',
      enforce: 'post',
      configResolved(config) {
        if (!config.server.https) {
          throw new Error('JFFI https=true requires server.https certificates or an SSL plugin in vite.config. See https://vite.dev/config/server-options#server-https');
        }
      }
    }]
  });
};
"#,
    )?;
    args.extend(["--config".into(), ".jffi-https.config.mjs".into()]);
    // Keep the application root as the configuration search directory.
    command.current_dir("platforms/web");
    Ok(())
}

/// Apply JFFI's browser target and deployment base to the generated Vite app.
pub(super) fn build_config() -> Result<()> {
    std::fs::write(
        "platforms/web/.jffi-build.config.mjs",
        r#"import { loadConfigFromFile, mergeConfig } from 'vite';
export default async (env) => {
  const original = await loadConfigFromFile(env);
  return mergeConfig(original?.config ?? {}, {
    base: process.env.JFFI_BASE_PATH,
    build: { target: process.env.JFFI_WEB_TARGET }
  });
};
"#,
    )?;
    Ok(())
}
