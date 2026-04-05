from typing import List
import random

class RuleMixer:
    """
    シミュレータ上で「既存ルールの合体」を行うための実験モジュール。
    JCrossの文脈で、例えば「リバーシの色の挟み込み」と「ブロック合わせ」を
    特定の割合で混合したワンライナーコード（仮説）を生成する。
    """
    
    BASE_RULES = {
        "color_fill": "もし LENGTH(PLAYER) > 0 { 色を塗る(0) }",
        "reversi_sandwich": "前方の色が自分と違う ならば 色を同じに変える()",
        "draw_line": "もし 移動可能 ならば 移動しながら色を変える()",
        "random_probe": "ランダムに選ぶ([0, 1, 2, 3])"
    }

    @staticmethod
    def generate_hybrid_rule(weights: dict = None) -> str:
        """
        指定された比率(weights)に基づいて既存の仮説ルールを結合した
        JCrossの関数テキストを返す
        例: {"color_fill": 0.5, "random_probe": 0.5}
        """
        if not weights:
            # 重みが指定されていなければランダムに2つ選んで結合
            keys = random.sample(list(RuleMixer.BASE_RULES.keys()), min(2, len(RuleMixer.BASE_RULES)))
        else:
            keys = list(weights.keys())

        # 実装としては優先度付きの if-else 分岐としてハイブリッドさせる
        # または、OR 条件で結合する
        
        hybrid_code = "関数 ハイブリッドシミュレーション() {\n"
        for i, k in enumerate(keys):
            rule_text = RuleMixer.BASE_RULES.get(k, "返す -1")
            
            # TODO: 擬似的な文法ではなく、真のJCross文法へパースするためのプロンプト指示ベースの文字列
            hybrid_code += f"    // {k} のエッセンスを混合\n"
            hybrid_code += f"    {rule_text}\n"

        hybrid_code += "    返す -1\n}"
        return hybrid_code
