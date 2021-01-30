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
            ob.sendLine('☞ 사용법: [대상] [키] [값] 속성제거')
            return
        words = line.split(None, 3)
        target = ob.env.findObjName(words[0])
        if target == None:
            ob.sendLine('☞ 그런 대상이 없어요!')
            return
         
        if words[1] not in target.attr:
            ob.sendLine('☞ 키가 없습니다.')
            return
        else:
            if target.checkAttr(words[1], words[2]) == False:
                ob.sendLine('☞ 속성이 없습니다.')
                return
                
            target.delAttr(words[1], words[2])
            ob.sendLine('☞ 속성이 제거 되었습니다.')
        
