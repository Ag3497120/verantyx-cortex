from dataclasses import dataclass, field
from typing import Tuple, Set, List, Optional
import json

@dataclass
class ActionIntent:
    type: str
    target_colors: Set[int] = field(default_factory=set)
    target_pos: Optional[Tuple[int, int]] = None
    reason: str = ""

@dataclass
class Experience:
    position: Tuple[int, int]
    colors: Set[int]
    event: str  # 'blocked', 'opened', 'passed', 'collected', 'changed'
    action: int
    frame: int
    grid_state_hash: int

class CrossSpace:
    """体験を蓄積し、共鳴で仮説を生み出す（ボトムアップ型推論）"""
    
    def __init__(self):
        self.experiences: List[Experience] = []
        
    def add(self, exp: Experience):
        self.experiences.append(exp)
        
    def resonate(self, current: Experience) -> List[Tuple[float, Experience]]:
        """今の体験と共鳴する過去の体験を返す。
        共鳴 = 同じ色を共有 × 近い位置 × 補完的なイベント"""
        
        scores = []
        for past in self.experiences:
            if past.frame == current.frame:
                continue
                
            # 色の重なり — 同じ色が関わってる体験は共鳴する
            color_overlap = len(current.colors & past.colors)
            
            # イベントの補完性 — 'blocked'と'opened'は強く共鳴する
            complement = 1.0
            if current.event == 'blocked' and past.event == 'opened': complement = 5.0
            if current.event == 'blocked' and past.event == 'passed': complement = 3.0
            if current.event == 'changed' and past.event == 'changed': complement = 2.0
            
            score = color_overlap * complement
            if score > 0:
                scores.append((score, past))
        
        # スコア順にソートして返す
        return sorted(scores, key=lambda x: x[0], reverse=True)
    
    def collide(self, exp_a: Experience, exp_b: Experience) -> Optional[ActionIntent]:
        """2つの体験がぶつかって行動（衝動）が生まれる。"""
        if exp_a.event == 'blocked' and exp_b.event == 'opened':
            # Bで壁が開いた時、何色を踏んでた？
            trigger_colors = exp_b.colors - exp_a.colors  # Bにあって Aにない色 = トリガー色
            if trigger_colors:
                return ActionIntent(
                    type='seek_color', 
                    target_colors=trigger_colors,
                    reason=f"Color {list(exp_a.colors)} blocked me, but past experience showed it opened when involving {list(trigger_colors)}."
                )
        
        if exp_a.event == 'blocked' and exp_b.event == 'passed':
            # 同じような壁を前に通れた → その時の位置に行ってみろ
            return ActionIntent(
                type='go_to', 
                target_pos=exp_b.position,
                reason=f"Color {list(exp_a.colors)} blocked me here, but I passed it at {exp_b.position} before."
            )
            
        return None
