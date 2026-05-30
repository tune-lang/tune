const vscode = require("vscode");
const childProcess = require("child_process");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;
let output;

function activate(context) {
  output = vscode.window.createOutputChannel("Tune");
  context.subscriptions.push(output);
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
  context.subscriptions.push(
    vscode.commands.registerCommand("tune.restartLanguageServer", async () => {
      await stopClient();
      startClient(context);
    }),
    vscode.commands.registerCommand("tune.checkFile", () => {
      runCurrentFileCommand(["check"], "check");
    }),
    vscode.commands.registerCommand("tune.formatCheckFile", () => {
      runCurrentFileCommand(["fmt", "--check"], "format check");
    })
  );
}

function startClient(context) {
  const command = dynoPath();
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

function dynoPath() {
  return vscode.workspace.getConfiguration("tune").get("dynoPath", "dyno");
}

function tuneDocument() {
  const document = vscode.window.activeTextEditor && vscode.window.activeTextEditor.document;
  if (!document || document.languageId !== "tune" || document.uri.scheme !== "file") {
    vscode.window.showWarningMessage("Open a Tune file first.");
    return undefined;
  }
  return document;
}

async function runCurrentFileCommand(args, label) {
  const document = tuneDocument();
  if (!document) {
    return;
  }
  await document.save();
  const commandArgs = [...args, document.uri.fsPath];
  output.clear();
  output.appendLine(`$ ${dynoPath()} ${commandArgs.join(" ")}`);
  childProcess.execFile(dynoPath(), commandArgs, (error, stdout, stderr) => {
    if (stdout) {
      output.append(stdout);
    }
    if (stderr) {
      output.append(stderr);
    }
    output.show(true);
    if (error) {
      vscode.window.showErrorMessage(`Tune ${label} failed.`);
    } else {
      vscode.window.showInformationMessage(`Tune ${label} passed.`);
    }
  });
}

function deactivate() {
  return stopClient();
}

module.exports = {
  activate,
  deactivate
};
