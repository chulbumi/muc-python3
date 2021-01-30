# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        words = line.split()
        if line == '' or len(words) < 2:
            ob.sendLine('☞ 사용법: [대상] [무공이름] 무공제거')
            return
        words = line.split(None, 1)
        target = ob.env.findObjName(words[0])
        if target == None:
            ob.sendLine('☞ 그런 대상이 없어요!')
            return
        for s in target.skills:
            if s.name == words[1]:
                ob.sendLine('☞ 무공이 제거되었습니다.')
                target.skills.remove(s)
                break
