import argparse
import json
import os
import sys

# Ensure api directory is in import search path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# Handle automatic compile on import failure or stale modules (e.g. missing AddFactRequest)
def compile_proto():
    proto_dir = os.path.dirname(os.path.abspath(__file__))
    proto_file = os.path.join(proto_dir, "daemon.proto")
    if os.path.exists(proto_file):
        try:
            from grpc_tools import protoc
            protoc.main((
                '',
                f'-I{proto_dir}',
                f'--python_out={proto_dir}',
                f'--grpc_python_out={proto_dir}',
                proto_file,
            ))
        except ImportError:
            print("Error: grpc_tools is not installed. Please run: pip install grpcio grpcio-tools", file=sys.stderr)
            sys.exit(1)

try:
    import daemon_pb2
    import daemon_pb2_grpc
    # Check if the imported proto contains the new AddFactRequest; if not, force recompile
    _ = daemon_pb2.AddFactRequest
except (ImportError, AttributeError):
    compile_proto()
    # Re-import after compile
    import daemon_pb2
    import daemon_pb2_grpc

import grpc

def get_grpc_stub():
    channel = grpc.insecure_channel('127.0.0.1:50051')
    return daemon_pb2_grpc.DaemonServiceStub(channel)

def run_cli_index(args):
    stub = get_grpc_stub()
    try:
        req = daemon_pb2.IndexRequest(repo_path=os.path.abspath(args.path))
        resp = stub.IndexRepository(req)
        print(f"Indexing completed: success={resp.success}")
        print(f"Indexed Symbols count: {resp.indexed_symbols}")
        print(f"Server Message: {resp.message}")
    except grpc.RpcError as e:
        print(f"gRPC Error: {e.details()} (Code: {e.code()})", file=sys.stderr)

def run_cli_add_fact(args):
    stub = get_grpc_stub()
    try:
        req = daemon_pb2.AddFactRequest(
            fact=args.fact,
            scope=args.scope,
            confidence=args.confidence
        )
        resp = stub.AddDurableFact(req)
        print(f"Fact added: success={resp.success}")
        print(f"Server Message: {resp.message}")
    except grpc.RpcError as e:
        print(f"gRPC Error: {e.details()} (Code: {e.code()})", file=sys.stderr)

def run_cli_context(args):
    stub = get_grpc_stub()
    try:
        req = daemon_pb2.ContextRequest(
            task_description=args.task,
            focus_files=args.files or []
        )
        resp = stub.GetContextForTask(req)
        print(f"Reasoning:\n{resp.reasoning}\n")
        print("Relevant Files Outlines:")
        for f in resp.files:
            print(f"--- File: {f.path} (Relevance: {f.relevance_score}) ---")
            print(f.snippet)
            print("-" * 40)
    except grpc.RpcError as e:
        print(f"gRPC Error: {e.details()} (Code: {e.code()})", file=sys.stderr)

def run_cli_verify(args):
    stub = get_grpc_stub()
    try:
        if os.path.exists(args.patch):
            with open(args.patch, 'r', encoding='utf-8') as f:
                patch_content = f.read()
        else:
            patch_content = args.patch

        req = daemon_pb2.VerifyRequest(
            patch_content=patch_content,
            target_branch=args.branch
        )
        resp = stub.VerifyPatch(req)
        print(f"Verification is_valid: {resp.is_valid}")
        print("\nCompile Errors:")
        for err in resp.compile_errors:
            print(err)
        print(f"\nLinter Output:\n{resp.linter_output}")
        print(f"\nTest Output:\n{resp.test_output}")
    except grpc.RpcError as e:
        print(f"gRPC Error: {e.details()} (Code: {e.code()})", file=sys.stderr)

def execute_mcp_tool(stub, tool_name, arguments):
    try:
        if tool_name == "index_repository":
            repo_path = arguments.get("repo_path")
            if not repo_path:
                return {"isError": True, "content": [{"type": "text", "text": "repo_path is required"}]}
            req = daemon_pb2.IndexRequest(repo_path=os.path.abspath(repo_path))
            resp = stub.IndexRepository(req)
            return {
                "content": [
                    {
                        "type": "text",
                        "text": f"Index status: success={resp.success}, indexed_symbols={resp.indexed_symbols}\nMessage: {resp.message}"
                    }
                ]
            }
            
        elif tool_name == "get_context":
            task_desc = arguments.get("task_description", "")
            focus_files = arguments.get("focus_files", [])
            req = daemon_pb2.ContextRequest(task_description=task_desc, focus_files=focus_files)
            resp = stub.GetContextForTask(req)
            
            files_info = []
            for f in resp.files:
                files_info.append(f"File: {f.path}\nRelevance: {f.relevance_score}\nSnippet Outline:\n{f.snippet}\n")
            
            output_text = f"Reasoning:\n{resp.reasoning}\n\n" + "\n".join(files_info)
            return {
                "content": [{"type": "text", "text": output_text}]
            }
            
        elif tool_name == "verify_patch":
            patch_content = arguments.get("patch_content")
            target_branch = arguments.get("target_branch", "main")
            if not patch_content:
                return {"isError": True, "content": [{"type": "text", "text": "patch_content is required"}]}
            req = daemon_pb2.VerifyRequest(patch_content=patch_content, target_branch=target_branch)
            resp = stub.VerifyPatch(req)
            
            output_text = f"Is Valid: {resp.is_valid}\n\nCompile Errors:\n" + "\n".join(resp.compile_errors)
            output_text += f"\n\nLinter Output:\n{resp.linter_output}\n\nTest Output:\n{resp.test_output}"
            return {
                "content": [{"type": "text", "text": output_text}]
            }
            
        else:
            return {"isError": True, "content": [{"type": "text", "text": f"Unknown tool: {tool_name}"}]}
    except Exception as e:
        return {"isError": True, "content": [{"type": "text", "text": f"gRPC server call failed: {str(e)}"}]}

def run_mcp_server():
    stub = get_grpc_stub()
    
    # Read/Write lines in stdio loop complying with MCP specification
    for line in sys.stdin:
        if not line:
            break
        try:
            req = json.loads(line)
            method = req.get("method")
            req_id = req.get("id")
            
            if method == "initialize":
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "atrium-mcp",
                            "version": "1.0.0"
                        }
                    }
                }
            elif method == "tools/list":
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "tools": [
                            {
                                "name": "index_repository",
                                "description": "Index a repository's source code files for Atrium.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "repo_path": {"type": "string", "description": "Absolute path to the repository"}
                                    },
                                    "required": ["repo_path"]
                                }
                            },
                            {
                                "name": "get_context",
                                "description": "Retrieve high-signal symbol structures and durable facts for a list of focused files.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "task_description": {"type": "string", "description": "The coding task description"},
                                        "focus_files": {
                                            "type": "array",
                                            "items": {"type": "string"},
                                            "description": "List of files to retrieve context for"
                                        }
                                    },
                                    "required": ["task_description", "focus_files"]
                                }
                            },
                            {
                                "name": "verify_patch",
                                "description": "Execute local verification checks (cargo check, cargo test, clippy) against a proposed patch.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "patch_content": {"type": "string", "description": "Unified diff patch content"},
                                        "target_branch": {"type": "string", "description": "Target branch to test against"}
                                    },
                                    "required": ["patch_content", "target_branch"]
                                }
                            }
                        ]
                    }
                }
            elif method == "tools/call":
                params = req.get("params", {})
                tool_name = params.get("name")
                arguments = params.get("arguments", {})
                
                result = execute_mcp_tool(stub, tool_name, arguments)
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": result
                }
            else:
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {}
                }
                
            sys.stdout.write(json.dumps(resp) + "\n")
            sys.stdout.flush()
        except Exception as e:
            sys.stderr.write(f"MCP Exception: {str(e)}\n")
            sys.stderr.flush()

def main():
    parser = argparse.ArgumentParser(description="Atrium Bridge Client / MCP Server")
    
    # We support either --mcp OR --action
    parser.add_argument("--mcp", action="store_true", help="Launch bridge in Model Context Protocol (MCP) server mode")
    parser.add_argument("--action", choices=["index", "add-fact", "get-context", "verify"], help="Action to execute")
    
    # Flags for various actions
    parser.add_argument("--path", help="Path to repository (used by index)")
    parser.add_argument("--fact", help="Fact/rule text to store (used by add-fact)")
    parser.add_argument("--scope", default="global", help="Fact scope (used by add-fact)")
    parser.add_argument("--confidence", type=float, default=1.0, help="Confidence score (used by add-fact)")
    parser.add_argument("--task", help="Coding task description (used by get-context)")
    parser.add_argument("--files", nargs="+", help="Focus file paths (optional, used by get-context)")
    parser.add_argument("--patch", help="Patch text or path to diff file (used by verify)")
    parser.add_argument("--branch", default="main", help="Target branch name (used by verify)")
    
    args = parser.parse_args()

    if args.mcp:
        run_mcp_server()
        return

    if not args.action:
        parser.print_help()
        sys.exit(1)

    if args.action == "index":
        if not args.path:
            print("Error: --path is required for index action", file=sys.stderr)
            sys.exit(1)
        run_cli_index(args)
    elif args.action == "add-fact":
        if not args.fact:
            print("Error: --fact is required for add-fact action", file=sys.stderr)
            sys.exit(1)
        run_cli_add_fact(args)
    elif args.action == "get-context":
        if not args.task:
            print("Error: --task is required for get-context action", file=sys.stderr)
            sys.exit(1)
        run_cli_context(args)
    elif args.action == "verify":
        if not args.patch:
            print("Error: --patch is required for verify action", file=sys.stderr)
            sys.exit(1)
        run_cli_verify(args)

if __name__ == "__main__":
    main()
