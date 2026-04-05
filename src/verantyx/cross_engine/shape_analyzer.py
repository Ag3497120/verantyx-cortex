from typing import List, Tuple, Set, Dict

class ShapeAnalyzer:
    """
    盤面全体を俯瞰し、隣接する同じ色のセル群を「形（Shape）」として抽出・認識するモジュール。
    CrossSpaceの初期状態（世界モデル）を構築するための第一段階。
    """
    
    @staticmethod
    def extract_shapes(grid: List[List[int]]) -> List[Dict]:
        if not grid or not grid[0]:
            return []
            
        rows = len(grid)
        cols = len(grid[0])
        visited: Set[Tuple[int, int]] = set()
        shapes = []
        shape_id_counter = 1
        
        for r in range(rows):
            for c in range(cols):
                if (r, c) not in visited:
                    color = grid[r][c]
                    
                    if color == 0:
                        visited.add((r, c))
                        continue
                        
                    shape_cells = ShapeAnalyzer._flood_fill(grid, r, c, color, visited)
                    if shape_cells:
                        min_r = min(cell[0] for cell in shape_cells)
                        max_r = max(cell[0] for cell in shape_cells)
                        min_c = min(cell[1] for cell in shape_cells)
                        max_c = max(cell[1] for cell in shape_cells)
                        
                        width = max_c - min_c + 1
                        height = max_r - min_r + 1
                        
                        shapes.append({
                            "id": f"S{shape_id_counter}",
                            "color": color,
                            "cells": set(shape_cells),
                            "bounding_box": (min_r, min_c, max_r, max_c),
                            "width": width,
                            "height": height,
                            "size": len(shape_cells)
                        })
                        shape_id_counter += 1
                        
        return sorted(shapes, key=lambda s: s["size"], reverse=True)

    @staticmethod
    def _flood_fill(grid: List[List[int]], start_r: int, start_c: int, color: int, visited: Set[Tuple[int, int]]) -> List[Tuple[int, int]]:
        rows = len(grid)
        cols = len(grid[0])
        cells = []
        stack = [(start_r, start_c)]
        
        while stack:
            r, c = stack.pop()
            if (r, c) in visited:
                continue
                
            if 0 <= r < rows and 0 <= c < cols and grid[r][c] == color:
                visited.add((r, c))
                cells.append((r, c))
                for dr, dc in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
                    stack.append((r + dr, c + dc))
                    
        return cells

    @staticmethod
    def build_relational_graph(grid: List[List[int]]) -> Dict:
        shapes = ShapeAnalyzer.extract_shapes(grid)
        nodes = []
        edges = []
        
        for s in shapes:
            # Nodes
            shape_type = "不規則"
            if s["size"] == 1: shape_type = "孤立点"
            elif s["width"] == 1 and s["height"] == s["size"]: shape_type = "垂直線"
            elif s["height"] == 1 and s["width"] == s["size"]: shape_type = "水平線"
            elif s["width"] * s["height"] == s["size"]: shape_type = "矩形"
            
            nodes.append({
                "id": s["id"],
                "color": s["color"],
                "type": shape_type,
                "size": s["size"],
                "bbox": s["bounding_box"]
            })
            
        # Edges
        for i in range(len(shapes)):
            for j in range(i + 1, len(shapes)):
                s1 = shapes[i]
                s2 = shapes[j]
                
                # Adjacency
                is_adjacent = False
                for r1, c1 in s1["cells"]:
                    for dr, dc in [(-1,0),(1,0),(0,-1),(0,1),(-1,-1),(-1,1),(1,-1),(1,1)]:
                        if (r1+dr, c1+dc) in s2["cells"]:
                            is_adjacent = True
                            break
                    if is_adjacent: break
                if is_adjacent:
                    edges.append({"source": s1["id"], "target": s2["id"], "relation": "Adjacent"})
                    
                # Containment
                # Condition: s1 bbox is completely inside s2 bbox (or vice versa)
                r1_min, c1_min, r1_max, c1_max = s1["bounding_box"]
                r2_min, c2_min, r2_max, c2_max = s2["bounding_box"]
                
                if r1_min >= r2_min and r1_max <= r2_max and c1_min >= c2_min and c1_max <= c2_max:
                    edges.append({"source": s2["id"], "target": s1["id"], "relation": "Contains"})
                elif r2_min >= r1_min and r2_max <= r1_max and c2_min >= c1_min and c2_max <= c1_max:
                    edges.append({"source": s1["id"], "target": s2["id"], "relation": "Contains"})
                    
                # Repetition (Same dimension & size)
                if s1["width"] == s2["width"] and s1["height"] == s2["height"] and s1["size"] == s2["size"]:
                    edges.append({"source": s1["id"], "target": s2["id"], "relation": "SameShape"})
                    
        return {"nodes": nodes, "edges": edges}

    @staticmethod
    def identify_patterns(shapes: List[Dict]) -> List[str]:
        # Backwards compatibility, now returning string representation of graph
        return []
