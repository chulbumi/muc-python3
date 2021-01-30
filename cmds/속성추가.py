# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        words = line.split()
        if line == '' or len(words) < 3:
            ob.sendLine('☞ 사용법: [대상] [키] [값] 키값설정')
            return
        words = line.split(None, 3)
        target = ob.env.findObjName(words[0])
        if target == None:
            ob.sendLine('☞ 그런 대상이 없어요!')
            return
        try:
            v = int(words[2])
        except:
            v = words[2]
         
        if words[1] not in target.attr:
            target[words[1]] = v
        else:
            try:
                target[words[1]] += '\r\n' + words[2]
            except:
                ob.sendLine('☞ 속성추가를 실패했습니다.')
                return
        ob.sendLine('☞ 속성이 추가 되었습니다.')
        