#!/usr/bin/env python3
"""
.jcross Japanese Parser (Stage 2.5)
完全日本語サポートのパーサー

日本語キーワード:
- 関数 (FUNCTION)
- 返す (RETURN)
- もし (IF)
- そうでなければ (ELSE)
- 各〜IN (FOR〜IN)
- 繰り返し (WHILE)
- 中断 (BREAK)
- 表示 (PRINT)
"""

import re
import sys
from pathlib import Path
from typing import Dict, List, Any, Optional


class JCrossJapaneseParser:
    """完全日本語対応.jcrossパーサー"""

    def __init__(self):
        self.globals = {}
        self.functions = {}
        self.return_value = None
        self.has_returned = False
        self.break_flag = False
        self.current_filepath = None
        self._init_builtins()

    def _init_builtins(self):
        """日本語組み込み関数を初期化"""
        import time
        import math

        # 日本語と英語の両方をサポート
        builtins_map = {
            # 日本語
            '長さ': lambda args: len(args[0]) if args else 0,
            '絶対値': lambda args: abs(args[0]) if args else 0,
            '現在時刻': lambda args: int(time.time()),
            '含む': lambda args: (args[1] in args[0]) if len(args) >= 2 else False,
            '大文字': lambda args: args[0].upper() if args and isinstance(args[0], str) else "",
            '小文字': lambda args: args[0].lower() if args and isinstance(args[0], str) else "",
            '分割': lambda args: args[0].split(args[1]) if len(args) >= 2 and isinstance(args[0], str) else [],
            '結合': lambda args: args[1].join(args[0]) if len(args) >= 2 and isinstance(args[0], list) else "",
            '最小': lambda args: min(args) if len(args) > 1 else (min(args[0]) if args and isinstance(args[0], list) else 0),
            '最大': lambda args: max(args) if len(args) > 1 else (max(args[0]) if args and isinstance(args[0], list) else 0),
            '合計': lambda args: sum(args[0]) if args and isinstance(args[0], list) else 0,
            '文字列': lambda args: str(args[0]) if args else "",
            '整数': lambda args: int(args[0]) if args else 0,
            '浮動小数': lambda args: float(args[0]) if args else 0.0,
            '平方根': lambda args: math.sqrt(args[0]) if args else 0,
            '四捨五入': lambda args: round(args[0], args[1]) if len(args) >= 2 else round(args[0]) if args else 0,
            '表示': lambda args: print(args[0]) if args else None,

            # ARC-AGI用 プランナー連携
            '経路探索': lambda args: __import__('planner').AStarPlanner(self.globals.get('FRONT.grid', [])).get_next_action(
                self.globals.get('PLAYER', []),
                args[0] if len(args) > 0 else [],
                args[1] if len(args) > 1 else []
            ),
            '位置検索': lambda args: next(([r, c] for r, row in enumerate(self.globals.get('FRONT.grid', [])) for c, val in enumerate(row) if val == args[0]), []),

            # 英語（互換性のため）
            'LENGTH': lambda args: len(args[0]) if args else 0,
            'len': lambda args: len(args[0]) if args else 0,
            'ABS': lambda args: abs(args[0]) if args else 0,
            'NOW': lambda args: int(time.time()),
            'CONTAINS': lambda args: (args[1] in args[0]) if len(args) >= 2 else False,
            'UPPER': lambda args: args[0].upper() if args and isinstance(args[0], str) else "",
            'LOWER': lambda args: args[0].lower() if args and isinstance(args[0], str) else "",
            'SPLIT': lambda args: args[0].split(args[1]) if len(args) >= 2 and isinstance(args[0], str) else [],
            'STR': lambda args: str(args[0]) if args else "",
            'INT': lambda args: int(args[0]) if args else 0,
            'FLOAT': lambda args: float(args[0]) if args else 0.0,
            'SQRT': lambda args: math.sqrt(args[0]) if args else 0,
            'ROUND': lambda args: round(args[0], args[1]) if len(args) >= 2 else round(args[0]) if args else 0,
            'SELF.save': lambda args: self.save(),
        }

        self.builtins = builtins_map

    def execute_file(self, filepath: str) -> Any:
        """
        .jcrossファイルを実行

        Args:
            filepath: .jcrossファイルのパス

        Returns:
            実行結果
        """
        print(f"🚀 日本語.jcross実行: {filepath}")

        self.current_filepath = filepath
        with open(filepath, 'r', encoding='utf-8') as f:
            source = f.read()

        # 実行
        try:
            result = self.execute(source)
            print("✅ 実行完了")
            return result
        except Exception as e:
            print(f"❌ エラー: {e}")
            import traceback
            traceback.print_exc()
            return None

    def execute(self, source: str) -> Any:
        """ソースコードを実行"""
        lines_raw = source.split('\n')
        lines = []

        # 1行完結の関数・パターン定義を展開
        for line in lines_raw:
            stripped = line.strip()
            if not stripped or stripped.startswith('//'):
                lines.append(stripped)
                continue
                
            # Geminiが出力しがちなワンライナー関数の展開
            if (stripped.startswith('関数 ') or stripped.startswith('PATTERN ') or stripped.startswith('FUNCTION ')) and stripped.endswith('}') and '{' in stripped:
                import re
                sub = re.sub(r'\{\s*(もし|IF|各|繰り返し|返す|RETURN)', r'{\n\1', stripped)
                sub = re.sub(r'\}\s*(もし|IF|各|繰り返し|返す|RETURN|そうでなければ)', r'}\n\1', sub)
                sub = re.sub(r'(?:返す|RETURN)\s+([^}]+?)\s*\}', r'返す \1\n}', sub)
                sub = re.sub(r'\s*\}\s*\}', r'\n}\n}', sub)
                
                for s_line in sub.split('\n'):
                    if s_line.strip():
                        lines.append(s_line.strip())
                continue
                
            lines.append(stripped)

        i = 0
        while i < len(lines):
            line = lines[i].strip()

            # コメントと空行をスキップ
            if not line or line.startswith('//'):
                i += 1
                continue

            # 文を実行
            i = self.execute_statement(lines, i)

            if self.has_returned:
                break

        return self.return_value

    def execute_statement(self, lines: List[str], start_idx: int) -> int:
        """文を実行し、次の行番号を返す"""
        line = lines[start_idx].strip()

        # 表示 (PRINT)
        if line.startswith('表示('):
            self.execute_print(line)
            return start_idx + 1

        # CROSS / AXIS
        if line.startswith('CROSS ') and line.endswith('{'):
            return self.execute_block_as_scope(lines, start_idx)
        if line.startswith('AXIS ') and line.endswith('{'):
            return self.execute_axis(lines, start_idx)
        # MATCH
        if line.startswith('MATCH ') and line.endswith('{'):
            return self.execute_match(lines, start_idx)
        # 関数定義 (PATTERN)
        if line.startswith('関数 ') or line.startswith('PATTERN ') or line.startswith('FUNCTION '):
            return self.execute_function_def(lines, start_idx)

        # もし (IF)
        if line.startswith('もし ') or line.startswith('IF '):
            return self.execute_if(lines, start_idx)

        # 繰り返し (WHILE true)
        if line == '繰り返し {':
            return self.execute_while(lines, start_idx)

        # 各〜IN (FOR IN)
        if line.startswith('各') and ' IN ' in line:
            return self.execute_for_in(lines, start_idx)

        # 返す (RETURN)
        if line.startswith('返す ') or line.startswith('RETURN '):
            self.execute_return(line)
            return start_idx + 1

        # 中断 (BREAK)
        if line == '中断':
            self.break_flag = True
            return start_idx + 1

        # リストへの追加: var に 追加(value)
        import re as _re
        _append_match = _re.match(r'(.+?)\s+に\s+追加\((.+)\)', line)
        if _append_match:
            list_name = _append_match.group(1).strip()
            value_expr = _append_match.group(2).strip()
            value = self.evaluate_expression(value_expr)
            target = self.globals.get(list_name)
            if isinstance(target, list):
                target.append(value)
            return start_idx + 1

        # 代入
        if '=' in line and not any(op in line for op in ['==', '!=', '>=', '<=', '>',' <']):
            self.execute_assignment(line)
            return start_idx + 1

        # その他の式
        self.evaluate_expression(line)
        return start_idx + 1

    def execute_print(self, line: str):
        """表示文を実行"""
        # 表示(...)から内容を抽出
        match = re.match(r'表示\((.*)\)', line)
        if match:
            expr = match.group(1)
            value = self.evaluate_expression(expr)
            print(value)

    def execute_assignment(self, line: str):
        """代入を実行"""
        # フィールドアクセス対応: obj.field = value
        if '.' in line.split('=')[0]:
            parts = line.split('=', 1)
            lhs = parts[0].strip()
            rhs = parts[1].strip()

            # obj.field.subfield = value の形式に対応
            path_parts = lhs.split('.')
            obj_name = path_parts[0]

            if obj_name not in self.globals:
                return

            obj = self.globals[obj_name]

            # 最後のフィールド以外をたどる
            for field in path_parts[1:-1]:
                if isinstance(obj, dict) and field in obj:
                    obj = obj[field]
                else:
                    return

            # 最後のフィールドに値を設定
            last_field = path_parts[-1]
            value = self.evaluate_expression(rhs)

            if isinstance(obj, dict):
                obj[last_field] = value
        else:
            # 通常の代入
            parts = line.split('=', 1)
            var_name = parts[0].strip()
            value_expr = parts[1].strip()
            value = self.evaluate_expression(value_expr)
            self.globals[var_name] = value

    def execute_function_def(self, lines: List[str], start_idx: int) -> int:
        """関数定義を実行"""
        line = lines[start_idx].strip()

        # 関数 name(param1, param2) {
        match = re.match(r'(?:関数|PATTERN|FUNCTION)\s+(\w+)\((.*?)\)\s*{', line)
        if not match:
            return start_idx + 1

        func_name = match.group(1)
        params_str = match.group(2)
        params = [p.strip() for p in params_str.split(',') if p.strip()]

        # 関数本体を見つける
        body_start = start_idx + 1
        body_end = self.find_matching_brace(lines, start_idx)

        # 関数を保存
        self.functions[func_name] = {
            'params': params,
            'body_lines': lines[body_start:body_end],
            'start': body_start,
            'end': body_end
        }

        # 関数をグローバル空間にも登録（呼び出し可能にする）
        def make_function(fn_name, fn_params, fn_body):
            def func(args):
                # 引数を設定
                local_vars = dict(self.globals)  # グローバル変数をコピー
                for i, param in enumerate(fn_params):
                    if i < len(args):
                        local_vars[param] = args[i]

                # 一時的にglobalsを置き換えて実行
                old_globals = self.globals
                self.globals = local_vars

                # 関数本体を実行
                old_return = self.has_returned
                old_return_value = self.return_value
                self.has_returned = False
                self.return_value = None

                # 複数行構造に対応するため、インデックスベースで実行
                i = 0
                while i < len(fn_body):
                    if self.has_returned:
                        break
                    i = self.execute_statement(fn_body, i)

                result = self.return_value

                # globalsを戻す
                self.globals = old_globals
                self.has_returned = old_return
                self.return_value = old_return_value

                return result

            return func

        # 呼び出し可能な関数を作成してglobalsに登録
        self.globals[func_name] = make_function(func_name, params, lines[body_start:body_end])

        return body_end + 1

    def execute_if(self, lines: List[str], start_idx: int) -> int:
        """もし文を実行"""
        line = lines[start_idx].strip()

        # もし condition {
        match = re.match(r'(?:もし|IF)\s+(.+?)\s*{', line)
        if not match:
            return start_idx + 1

        condition = match.group(1)
        cond_value = self.evaluate_expression(condition)

        # ブロックを見つける
        if_end = self.find_matching_brace(lines, start_idx)

        # else句を探す
        else_start = None
        if if_end < len(lines) and lines[if_end].strip().startswith('} そうでなければ'):
            else_start = if_end
            # else句の終わりを見つける
            if lines[else_start].strip() == '} そうでなければ {':
                else_end = self.find_matching_brace(lines, else_start)
            else:
                else_end = else_start + 1

        if cond_value:
            # if句を実行
            i = start_idx + 1
            while i < if_end:
                if self.has_returned or self.break_flag:
                    break
                i = self.execute_statement(lines, i)
            return else_end + 1 if else_start else if_end + 1
        else:
            # else句があれば実行
            if else_start:
                i = else_start + 1
                while i < else_end:
                    if self.has_returned or self.break_flag:
                        break
                    i = self.execute_statement(lines, i)
                return else_end + 1
            return if_end + 1

    def execute_while(self, lines: List[str], start_idx: int) -> int:
        """繰り返し文を実行"""
        loop_end = self.find_matching_brace(lines, start_idx)

        while True:
            if self.has_returned:
                break

            # ブロックを実行
            i = start_idx + 1
            while i < loop_end:
                if self.has_returned or self.break_flag:
                    break
                i = self.execute_statement(lines, i)

            if self.break_flag:
                self.break_flag = False
                break

        return loop_end + 1

    def execute_for_in(self, lines: List[str], start_idx: int) -> int:
        """各〜IN文を実行"""
        line = lines[start_idx].strip()

        # 各item IN collection {
        match = re.match(r'各(\S+)\s+IN\s+(.+?)\s*{', line)
        if not match:
            return start_idx + 1

        item_var = match.group(1)
        collection_expr = match.group(2)
        collection = self.evaluate_expression(collection_expr)

        loop_end = self.find_matching_brace(lines, start_idx)

        if isinstance(collection, list):
            for item in collection:
                if self.has_returned or self.break_flag:
                    break

                self.globals[item_var] = item

                # ブロックを実行
                i = start_idx + 1
                while i < loop_end:
                    if self.has_returned or self.break_flag:
                        break
                    i = self.execute_statement(lines, i)

        if self.break_flag:
            self.break_flag = False

        return loop_end + 1

    def execute_return(self, line: str):
        """返す文を実行"""
        match = re.match(r'(?:返す|RETURN)\s+(.+)', line)
        if match:
            expr = match.group(1)
            self.return_value = self.evaluate_expression(expr)
            self.has_returned = True

    def evaluate_expression(self, expr: str) -> Any:
        """式を評価"""
        expr = expr.strip()

        # 括弧でくくられた式: (expr) — ただし関数呼び出しと区別する
        if expr.startswith('(') and expr.endswith(')') and '(' not in expr[1:-1]:
            return self.evaluate_expression(expr[1:-1])

        # 文字列リテラル
        if expr.startswith('"') and expr.endswith('"'):
            return expr[1:-1]

        # 数値リテラル
        try:
            if '.' in expr:
                return float(expr)
            return int(expr)
        except ValueError:
            pass

        # 真偽値
        if expr == '真':
            return True
        if expr == '偽':
            return False

        # リストリテラル
        if expr.startswith('[') and expr.endswith(']'):
            return self.parse_list_literal(expr)

        # 辞書リテラル
        if expr.startswith('{') and expr.endswith('}'):
            return self.parse_dict_literal(expr)

        # 関数呼び出し（最優先）— ただし (expr) % op のような括弧式は除く
        if '(' in expr and ')' in expr:
            # 関数呼び出しパターン: word(args) — 先頭が識別子の場合のみ
            import re as _re2
            if _re2.match(r'[\w\u3000-\u9fff]+\(', expr):
                result = self.call_function(expr)
                if result is not None:
                    return result
                # call_functionが失敗したら演算子チェックに落ちる

        # 論理演算子 (最低優先度: または / OR)
        for op in [' または ', ' OR ']:
            if op in expr:
                parts = expr.split(op, 1)
                return self.evaluate_expression(parts[0].strip()) or self.evaluate_expression(parts[1].strip())

        # 論理演算子 (最低優先度: そして / かつ / AND)
        for op in [' そして ', ' かつ ', ' AND ']:
            if op in expr:
                parts = expr.split(op, 1)
                left_val = self.evaluate_expression(parts[0].strip())
                if not left_val:
                    return False
                return bool(self.evaluate_expression(parts[1].strip()))

        # 二項演算（比較演算子を優先）
        for op in ['>=', '<=', '==', '!=']:
            if op in expr:
                return self.evaluate_binary_op(expr, op)

        # 比較演算子（単独）
        for op in ['>', '<']:
            if op in expr and '>=' not in expr and '<=' not in expr:
                return self.evaluate_binary_op(expr, op)

        # 算術演算子（括弧の外にある演算子を探す）
        for op in ['%', '+', '-', '*', '/']:
            if op in expr and self._has_op_outside_parens(expr, op):
                return self.evaluate_binary_op(expr, op)

        # リスト/辞書のインデックスアクセス: obj[0] or obj["key"]
        if '[' in expr and expr.endswith(']'):
            import re
            match = re.match(r'^(.*)\[([^\[\]]+)\]$', expr)
            if match:
                obj_expr = match.group(1).strip()
                index_expr = match.group(2).strip()

                obj = self.evaluate_expression(obj_expr)
                index = self.evaluate_expression(index_expr)

                if isinstance(obj, (list, dict)):
                    try:
                        return obj[index]
                    except (KeyError, IndexError, TypeError):
                        return None

        # 変数参照（直接キー名で存在するか）
        if expr in self.globals:
            return self.globals[expr]

        # フィールドアクセス: obj.field
        if '.' in expr and not expr.startswith('"'):
            parts = expr.split('.')
            obj = self.globals.get(parts[0])

            for field in parts[1:]:
                if isinstance(obj, dict):
                    obj = obj.get(field)
                else:
                    break

            return obj

        # どれにも一致しなければ文字列そのものを返す（あるいは None を返す）
        return self.globals.get(expr, expr)

    def _has_op_outside_parens(self, expr: str, op: str) -> bool:
        """括弧の外に演算子があるか確認"""
        depth = 0
        i = 0
        while i < len(expr):
            ch = expr[i]
            if ch in '([{':
                depth += 1
            elif ch in ')]}':
                depth -= 1
            elif depth == 0 and expr[i:i+len(op)] == op:
                # 演算子が文字列リテラルの外にあるか確認
                if i > 0:  # 先頭でなければ二項演算子として有効
                    return True
            i += 1
        return False

    def _split_on_op_outside_parens(self, expr: str, op: str):
        """括弧の外の最後の演算子で分割（左結合のため）"""
        depth = 0
        last_pos = -1
        i = 0
        while i < len(expr):
            ch = expr[i]
            if ch in '([{':
                depth += 1
            elif ch in ')]}':
                depth -= 1
            elif depth == 0 and expr[i:i+len(op)] == op:
                if i > 0:
                    last_pos = i
            i += 1
        if last_pos == -1:
            return None
        return expr[:last_pos].strip(), expr[last_pos+len(op):].strip()

    def evaluate_binary_op(self, expr: str, op: str) -> Any:
        """二項演算を評価"""
        # 比較演算子を先に処理
        if op in ['>=', '<=', '==', '!=', '>', '<']:
            parts = expr.split(op, 1)
            if len(parts) != 2:
                return None

            left = self.evaluate_expression(parts[0].strip())
            right = self.evaluate_expression(parts[1].strip())

            # None値の安全な処理（比較演算）
            if left is None and right is None:
                return True if op in ['==', '>=', '<='] else False
            if left is None:
                left = 0
            if right is None:
                right = 0

            if op == '>':
                return left > right
            elif op == '<':
                return left < right
            elif op == '>=':
                return left >= right
            elif op == '<=':
                return left <= right
            elif op == '==':
                return left == right
            elif op == '!=':
                return left != right

        # 算術演算子（括弧を考慮した分割）
        split_result = self._split_on_op_outside_parens(expr, op)
        if split_result is None:
            parts = expr.split(op, 1)
            if len(parts) != 2:
                return None
            left_str, right_str = parts[0].strip(), parts[1].strip()
        else:
            left_str, right_str = split_result

        left = self.evaluate_expression(left_str)
        right = self.evaluate_expression(right_str)

        # リストや文字列の場合はそのまま処理
        if op == '+':
            # リスト・文字列の連結をサポート
            if isinstance(left, list) and isinstance(right, list):
                return left + right
            if isinstance(left, str) or isinstance(right, str):
                return str(left) + str(right)

        # None値の安全な処理（数値演算のみ）
        if left is None:
            left = 0
        if right is None:
            right = 0

        # 文字列を数値に変換を試みる
        if isinstance(left, str):
            try:
                left = float(left) if '.' in left else int(left)
            except:
                pass

        if isinstance(right, str):
            try:
                right = float(right) if '.' in right else int(right)
            except:
                pass

        if op == '+':
            return left + right
        elif op == '-':
            return left - right
        elif op == '*':
            return left * right
        elif op == '/':
            if isinstance(left, (int, float)) and isinstance(right, (int, float)):
                return left / right if right != 0 else 0
            return 0
        elif op == '%':
            if isinstance(left, (int, float)) and isinstance(right, (int, float)) and right != 0:
                return int(left) % int(right)
            return 0

        return None

    def parse_list_literal(self, expr: str) -> list:
        """リストリテラルをパース"""
        # [ ] を除去
        content = expr[1:-1].strip()
        if not content:
            return []

        result = []
        # 要素を分割（ネストに対応）
        elements = []
        current = ""
        depth = 0

        for char in content:
            if char in '{[':
                depth += 1
                current += char
            elif char in '}]':
                depth -= 1
                current += char
            elif char == ',' and depth == 0:
                elements.append(current.strip())
                current = ""
            else:
                current += char

        if current.strip():
            elements.append(current.strip())

        # 各要素を評価
        for elem in elements:
            value = self.evaluate_expression(elem)
            result.append(value)

        return result

    def parse_dict_literal(self, expr: str) -> dict:
        """辞書リテラルをパース"""
        # { } を除去
        content = expr[1:-1].strip()
        if not content:
            return {}

        result = {}
        # キー:値 のペアを分割
        # ネストした辞書・リストに対応するため、カンマの深さを追跡
        pairs = []
        current = ""
        depth = 0

        for char in content:
            if char in '{[':
                depth += 1
                current += char
            elif char in '}]':
                depth -= 1
                current += char
            elif char == ',' and depth == 0:
                pairs.append(current.strip())
                current = ""
            else:
                current += char

        if current.strip():
            pairs.append(current.strip())

        # 各ペアを処理
        for pair in pairs:
            if ':' not in pair:
                continue

            # キーと値を分割（最初の:で分割）
            colon_pos = pair.index(':')
            key = pair[:colon_pos].strip()
            value_expr = pair[colon_pos+1:].strip()

            # キーを評価（変数の場合もある）
            if key.startswith('"') or key.startswith("'"):
                key = key[1:-1]  # 文字列リテラルの場合
            # else: 変数名やその他のキーはそのまま使用

            # 値を評価
            value = self.evaluate_expression(value_expr)
            result[key] = value

        return result

    def call_function(self, expr: str) -> Any:
        """関数呼び出しを実行"""
        match = re.match(r'^([\w\.]+)\((.*)\)$', expr, re.DOTALL)
        if not match:
            return None

        func_name = match.group(1)
        args_str = match.group(2)

        # 引数を評価
        args = []
        if args_str.strip():
            # 簡易実装: カンマで分割（ネストを考慮しない）
            arg_parts = self.split_arguments(args_str)
            for arg in arg_parts:
                result = self.evaluate_expression(arg.strip())
                args.append(result)

        # 組み込み関数
        if func_name in self.builtins:
            return self.builtins[func_name](args)

        # ユーザー定義関数
        if func_name in self.functions:
            func = self.functions[func_name]

            # パラメータをバインド（一時的にローカル変数として追加）
            temp_params = {}
            for i, param in enumerate(func['params']):
                if i < len(args):
                    temp_params[param] = args[i]
                    self.globals[param] = args[i]

            # 関数本体を実行
            old_return = self.has_returned
            old_return_value = self.return_value
            self.has_returned = False
            self.return_value = None

            i = 0
            while i < len(func['body_lines']):
                if self.has_returned:
                    break
                i = self.execute_statement(func['body_lines'], i)

            result = self.return_value

            # 状態を復元（パラメータを削除）
            self.has_returned = old_return
            self.return_value = old_return_value
            for param in temp_params:
                if param in self.globals:
                    del self.globals[param]

            return result

        return None

    def split_arguments(self, args_str: str) -> List[str]:
        """関数引数を分割（ネストを考慮）"""
        parts = []
        current = ""
        depth = 0

        for char in args_str:
            if char in '([{':
                depth += 1
                current += char
            elif char in ')]}':
                depth -= 1
                current += char
            elif char == ',' and depth == 0:
                parts.append(current.strip())
                current = ""
            else:
                current += char

        if current.strip():
            parts.append(current.strip())

        return parts

    def find_matching_brace(self, lines: List[str], start_idx: int) -> int:
        """対応する}を見つける"""
        depth = 1
        i = start_idx + 1

        while i < len(lines) and depth > 0:
            line = lines[i].strip()

            # コメントをスキップ
            if line.startswith('//'):
                i += 1
                continue

            depth += line.count('{')
            depth -= line.count('}')

            if depth == 0:
                return i

            i += 1

        return i



    def execute_block_as_scope(self, lines, start_idx):
        block_end = self.find_matching_brace(lines, start_idx)
        i = start_idx + 1
        while i < block_end:
            if self.has_returned or self.break_flag: break
            i = self.execute_statement(lines, i)
        return block_end + 1

    def execute_axis(self, lines, start_idx):
        line = lines[start_idx].strip()
        axis_name = re.match(r'AXIS\s+(\w+)\s*{', line).group(1)
        block_end = self.find_matching_brace(lines, start_idx)
        axis_data = {}
        for i in range(start_idx + 1, block_end):
            ln = lines[i].strip()
            if not ln or ln.startswith('//'): continue
            if ':' in ln:
                k, v = ln.split(':', 1)
                k = k.strip()
                if k.startswith('"') or k.startswith("'"): k = k[1:-1]
                axis_data[k] = self.evaluate_expression(v.strip())
        self.globals[axis_name] = axis_data
        return block_end + 1

    def execute_match(self, lines, start_idx):
        line = lines[start_idx].strip()
        target_expr = re.match(r'MATCH\s+(.+?)\s*{', line).group(1).strip()
        match_target = self.evaluate_expression(target_expr)
        block_end = self.find_matching_brace(lines, start_idx)
        i = start_idx + 1
        while i < block_end:
            ln = lines[i].strip()
            if not ln or ln.startswith('//'):
                i += 1; continue
            if '->' in ln:
                pat, res = [x.strip() for x in ln.split('->', 1)]
                is_match = False
                if pat == 'DEFAULT': is_match = True
                elif pat.startswith('CONTAINS '):
                    args = self.evaluate_expression(pat[9:].strip())
                    is_match = any(str(arg) in str(match_target) for arg in args) if isinstance(args, list) else str(args) in str(match_target)
                elif pat.startswith('HAS '):
                    args = self.evaluate_expression(pat[4:].strip())
                    is_match = any(arg in match_target for arg in args) if isinstance(args, list) else args in match_target
                else:
                    is_match = (self.evaluate_expression(pat) == match_target)
                
                if is_match:
                    if res == '{':
                        res_end = self.find_matching_brace(lines, i)
                        j = i + 1
                        while j < res_end:
                            if self.has_returned or self.break_flag: break
                            j = self.execute_statement(lines, j)
                        return block_end + 1
                    else:
                        self.return_value = self.evaluate_expression(res)
                        self.has_returned = True
                        return block_end + 1
            i += 1
        return block_end + 1

    def save(self):
        if not self.current_filepath: return None
        with open(self.current_filepath, 'r', encoding='utf-8') as f:
            lines = f.read().split('\n')
        new_lines, i, import_json = [], 0, __import__('json')
        while i < len(lines):
            line = lines[i]
            match = re.match(r'AXIS\s+(\w+)\s*{', line.strip())
            if match:
                axis_name = match.group(1)
                new_lines.append(line)
                block_end = self.find_matching_brace(lines, i)
                axis_data = self.globals.get(axis_name, {})
                for k, v in axis_data.items():
                    val_str = import_json.dumps(v, ensure_ascii=False)
                    new_lines.append(f"        {k}: {val_str}")
                new_lines.append("    }")
                i = block_end
            else:
                new_lines.append(line)
            i += 1
        with open(self.current_filepath, 'w', encoding='utf-8') as f:
            f.write('\n'.join(new_lines))
        return True


if __name__ == '__main__':

    if len(sys.argv) < 2:
        print("使い方: python3 jcross_japanese_parser.py <file.jcross>")
        sys.exit(1)

    parser = JCrossJapaneseParser()
    parser.execute_file(sys.argv[1])

