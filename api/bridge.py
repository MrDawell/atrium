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
    # Check if the imported proto contains the new SearchRequest; if not, force recompile
    _ = daemon_pb2.SearchRequest
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

        elif tool_name == "atrium_search_symbols":
            query = arguments.get("query")
            if not query:
                return {"isError": True, "content": [{"type": "text", "text": "query is required"}]}
            req = daemon_pb2.SearchRequest(query=query)
            resp = stub.SearchSymbols(req)
            if not resp.success:
                return {"isError": True, "content": [{"type": "text", "text": resp.message}]}
            
            matches_info = []
            for m in resp.matches:
                matches_info.append(f"File: {m.file_path} | Name: {m.symbol_name} ({m.symbol_kind})")
            
            output_text = f"Matches:\n" + "\n".join(matches_info) if matches_info else "No matching symbols found."
            return {
                "content": [{"type": "text", "text": output_text}]
            }

        elif tool_name == "atrium_get_precision_slice":
            file_path = arguments.get("file_path")
            task_context = arguments.get("task_context", "")
            if not file_path:
                return {"isError": True, "content": [{"type": "text", "text": "file_path is required"}]}
            req = daemon_pb2.SliceRequest(file_path=file_path, task_context=task_context)
            resp = stub.GetPrecisionSlice(req)
            if not resp.success:
                return {"isError": True, "content": [{"type": "text", "text": resp.message}]}
            return {
                "content": [{"type": "text", "text": resp.code_slice}]
            }

        elif tool_name == "atrium_verify_patch":
            patch_content = arguments.get("patch_diff")
            if patch_content is None:
                patch_content = ""
            target_branch = arguments.get("target_branch", "main")
            req = daemon_pb2.VerifyRequest(patch_content=patch_content, target_branch=target_branch)
            resp = stub.VerifyPatch(req)
            
            output_text = f"Is Valid: {resp.is_valid}\n\nCompile Errors:\n" + "\n".join(resp.compile_errors)
            output_text += f"\n\nLinter Output:\n{resp.linter_output}\n\nTest Output:\n{resp.test_output}"
            return {
                "content": [{"type": "text", "text": output_text}]
            }
            
        else:
            return {"isError": True, "content": [{"type": "text", "text": f"Unknown tool: {tool_name}"}]}
    except grpc.RpcError as e:
        details = e.details() if hasattr(e, 'details') and e.details() else str(e)
        if e.code() == grpc.StatusCode.UNAVAILABLE or "Symbol not indexed" in details or "not indexed yet" in details:
            err_msg = "Error: Symbol not indexed yet. Please run atrium_search_symbols first to warm the cache."
        else:
            err_msg = f"gRPC server call failed: {details}"
        return {"isError": True, "content": [{"type": "text", "text": err_msg}]}
    except Exception as e:
        err_msg = str(e)
        if "Symbol not indexed" in err_msg or "not indexed yet" in err_msg:
            err_msg = "Error: Symbol not indexed yet. Please run atrium_search_symbols first to warm the cache."
        return {"isError": True, "content": [{"type": "text", "text": f"Error: {err_msg}"}]}

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
                            },
                            {
                                "name": "atrium_search_symbols",
                                "description": "Search the local AST symbol graph for class, function, struct, or method signatures matching a query. Use this BEFORE reading raw files to locate where logic lives. You have persistent repository memory via an underlying SQLite database.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "query": {"type": "string", "description": "The search query (e.g., class, function, struct, or method name)"}
                                    },
                                    "required": ["query"]
                                }
                            },
                            {
                                "name": "atrium_get_precision_slice",
                                "description": "Retrieve an optimized, token-saving structural code slice (signatures + relevant rules) for a specific file path. Use this instead of dumping the full file to save 98% in token costs. You have persistent repository memory via an underlying SQLite database.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "file_path": {"type": "string", "description": "Absolute or relative file path to retrieve structural slice for"},
                                        "task_context": {"type": "string", "description": "Context of the task being performed"}
                                    },
                                    "required": ["file_path", "task_context"]
                                }
                            },
                            {
                                "name": "atrium_verify_patch",
                                "description": "Trigger the local background verification pipeline (cargo check, tests, clippy) to validate your proposed code modifications. Always run this tool AFTER modifying files to guarantee you didn't break the build before finalizing your response. You have persistent repository memory via an underlying SQLite database. If a compilation or lint error occurs during atrium_verify_patch, you must autonomously analyze the compiler feedback and call the tools again to fix your own mistakes before presenting the final answer to the user.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "patch_diff": {"type": "string", "description": "Unified diff patch content representing proposed modifications (optional)"}
                                    },
                                    "required": []
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
                
            if req_id is not None:
                sys.stdout.write(json.dumps(resp) + "\n")
                sys.stdout.flush()
        except Exception as e:
            sys.stderr.write(f"MCP Exception: {str(e)}\n")
            sys.stderr.flush()

def run_cli_search_symbols(args):
    stub = get_grpc_stub()
    try:
        req = daemon_pb2.SearchRequest(query=args.query)
        resp = stub.SearchSymbols(req)
        if not resp.success:
            print(f"Search failed: {resp.message}")
            return
        print(f"Search results for '{args.query}':")
        for m in resp.matches:
            print(f" - File: {m.file_path} | Name: {m.symbol_name} ({m.symbol_kind})")
    except grpc.RpcError as e:
        print(f"gRPC Error: {e.details()} (Code: {e.code()})", file=sys.stderr)

def run_cli_get_precision_slice(args):
    stub = get_grpc_stub()
    try:
        req = daemon_pb2.SliceRequest(file_path=args.file_path, task_context=args.task_context or "")
        resp = stub.GetPrecisionSlice(req)
        if not resp.success:
            print(f"Slice failed: {resp.message}")
            return
        print(resp.code_slice)
    except grpc.RpcError as e:
        print(f"gRPC Error: {e.details()} (Code: {e.code()})", file=sys.stderr)

def main():
    # Normalize "mcp" positional argument to "--mcp" flag
    for idx, arg in enumerate(sys.argv):
        if arg == "mcp":
            sys.argv[idx] = "--mcp"

    parser = argparse.ArgumentParser(description="Atrium Bridge Client / MCP Server")
    
    # We support either --mcp OR --action
    parser.add_argument("--mcp", action="store_true", help="Launch bridge in Model Context Protocol (MCP) server mode")
    parser.add_argument("--action", choices=["index", "add-fact", "get-context", "verify", "search-symbols", "get-precision-slice"], help="Action to execute")
    
    # Flags for various actions
    parser.add_argument("--path", help="Path to repository (used by index)")
    parser.add_argument("--fact", help="Fact/rule text to store (used by add-fact)")
    parser.add_argument("--scope", default="global", help="Fact scope (used by add-fact)")
    parser.add_argument("--confidence", type=float, default=1.0, help="Confidence score (used by add-fact)")
    parser.add_argument("--task", help="Coding task description (used by get-context)")
    parser.add_argument("--files", nargs="+", help="Focus file paths (optional, used by get-context)")
    parser.add_argument("--patch", help="Patch text or path to diff file (used by verify)")
    parser.add_argument("--branch", default="main", help="Target branch name (used by verify)")
    parser.add_argument("--query", help="Query string (used by search-symbols)")
    parser.add_argument("--file-path", help="Target file path (used by get-precision-slice)")
    parser.add_argument("--task-context", help="Task context description (used by get-precision-slice)")
    
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
    elif args.action == "search-symbols":
        if not args.query:
            print("Error: --query is required for search-symbols action", file=sys.stderr)
            sys.exit(1)
        run_cli_search_symbols(args)
    elif args.action == "get-precision-slice":
        if not args.file_path:
            print("Error: --file-path is required for get-precision-slice action", file=sys.stderr)
            sys.exit(1)
        run_cli_get_precision_slice(args)

if __name__ == "__main__":
    main()
