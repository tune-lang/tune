const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;

function activate(context) {
  startClient(context);
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async event => {
      if (!event.affectsConfiguration("tune.dynoPath")) {
        return;
      }
      await stopClient();
      startClient(context);
    })
  );
}

function startClient(context) {
  const command = vscode.workspace.getConfiguration("tune").get("dynoPath", "dyno");
  client = new LanguageClient("tune", "Tune Language Server", serverOptions(command), clientOptions);
  context.subscriptions.push(client.start());
}

async function stopClient() {
  if (!client) {
    return undefined;
  }
  const stopped = client.stop();
  client = undefined;
  return stopped;
}

function serverOptions(command) {
  return {
    run: { command, args: ["lsp"], transport: TransportKind.stdio },
    debug: { command, args: ["lsp"], transport: TransportKind.stdio }
  };
}

const clientOptions = {
  documentSelector: [{ scheme: "file", language: "tune" }],
  synchronize: {
    fileEvents: vscode.workspace.createFileSystemWatcher("**/*.tn")
  }
};

function deactivate() {
  return stopClient();
}

module.exports = {
  activate,
  deactivate
};
