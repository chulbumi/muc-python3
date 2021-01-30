# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        from objs.rank import Rank, RANK
        if line == '':
            RANK.attr = {}
            RANK.save()
            ob.sendLine('* 전체가 초기화 되었습니다.')
            return
        
        if line not in RANK.attr:
            ob.sendLine('☞ 그런 순위는 없습니다.')
            return
        RANK.attr[line] = []
        RANK.save()
        ob.sendLine('* 초기화 되었습니다.')
