import heapq
from typing import List, Tuple, Set

class AStarPlanner:
    """
    A*アルゴリズムを用いた経路探索プランナー。
    盤面全体の2D配列を受け取り、障害物として指定された色を避けながら
    目標座標までの最短経路を見つけ、その最初の行動（0=上, 1=右, 2=下, 3=左）を返す。
    """
    def __init__(self, grid: List[List[int]]):
        self.grid = grid
        self.height = len(grid)
        self.width = len(grid[0]) if self.height > 0 else 0

    def heuristic(self, current_pos: Tuple[int, int], target_pos: Tuple[int, int]) -> int:
        return abs(current_pos[0] - target_pos[0]) + abs(current_pos[1] - target_pos[1])

    def get_next_action(self, start_pos: List[int], target_pos: List[int], blocked_colors: List[int]) -> int:
        if not start_pos or not target_pos or len(start_pos) < 2 or len(target_pos) < 2:
            return -1
        
        start = (start_pos[0], start_pos[1])
        target = (target_pos[0], target_pos[1])

        # すでに目標にいる場合は停止（待機 = 5）またはアクション（4）、文脈によるがプランナーとしては到達済み（5を返すか、エラー）を返す
        if start == target:
            return 5 
        
        open_set = []
        heapq.heappush(open_set, (0, start))
        
        came_from = {}
        # (アクション, 遷移元座標) を保存する。これを使って後で経路を逆順に辿る
        g_score = {start: 0}
        f_score = {start: self.heuristic(start, target)}
        
        blocked_set = set(blocked_colors)

        # 動ける方向とアクションのマッピング (0:Up, 1:Right, 2:Down, 3:Left)
        # 上: y-1, 右: x+1, 下: y+1, 左: x-1
        directions = [
            (-1, 0, 0),  # d_row, d_col, action_code
            (0, 1, 1),
            (1, 0, 2),
            (0, -1, 3)
        ]

        while open_set:
            _, current = heapq.heappop(open_set)

            if current == target:
                return self.reconstruct_first_action(came_from, current, start)

            for d_row, d_col, action_code in directions:
                neighbor = (current[0] + d_row, current[1] + d_col)

                # 盤面外チェック
                if neighbor[0] < 0 or neighbor[0] >= self.height or neighbor[1] < 0 or neighbor[1] >= self.width:
                    continue

                # 目標座標がブロック色であっても、最後の1手として飛び込めるようにする（例えば鍵の色がblockedとして渡されてしまう可能性の回避）
                # 通常は Target == current のときに終わるが、Targetそのものが blocked color と同色の場合がある。
                if neighbor != target:
                    color = self.grid[neighbor[0]][neighbor[1]]
                    if color in blocked_set:
                        continue

                tentative_g_score = g_score[current] + 1
                
                if neighbor not in g_score or tentative_g_score < g_score[neighbor]:
                    came_from[neighbor] = (current, action_code)
                    g_score[neighbor] = tentative_g_score
                    f_score[neighbor] = tentative_g_score + self.heuristic(neighbor, target)
                    
                    if neighbor not in [i[1] for i in open_set]:
                        heapq.heappush(open_set, (f_score[neighbor], neighbor))

        # 到達不可能な場合
        return -1
        
    def reconstruct_first_action(self, came_from, current, start) -> int:
        """
        目標座標から逆順に辿り、スタート直後の最初のアクションを返す
        """
        last_action = -1
        while current in came_from:
            prev, action = came_from[current]
            if prev == start:
                return action
            current = prev
            last_action = action
        return last_action
