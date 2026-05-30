const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;

function activate(context) {
  const config = vscode.workspace.getConfiguration("tune");
  const command = config.get("dynoPath", "dyno");
  const serverOptions = {
    run: { command, args: ["lsp"], transport: TransportKind.stdio },
    debug: { command, args: ["lsp"], transport: TransportKind.stdio }
  };
  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "tune" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.tn")
    }
  };

  client = new LanguageClient("tune", "Tune Language Server", serverOptions, clientOptions);
  context.subscriptions.push(client.start());
}

function deactivate() {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

module.exports = {
  activate,
  deactivate
};
