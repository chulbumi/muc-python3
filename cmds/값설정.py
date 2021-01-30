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
            ob.sendLine('☞ 사용법: [대상] [키] [값] 값설정')
            return
        #ob.sendLine('☞ 공사중입니다.')
        #return
        if len(words[2]) > 50:
            ob.sendLine('☞ 너무 길어요!')
            return
        words = line.split(None, 2)
        if words[0] == '방':
            target = ob.env
        else:
            target = ob.env.findObjName(words[0])

        if target == None:
            target = ob.findObjName(words[0])
            if target == None:
                ob.sendLine('☞ 그런 대상이 없어요!')
                return
        """    
        if ob['관리자등급'] < 2000:
            if words[1] not in target.attr:
                ob.sendLine('☞ 해당 키가 없습니다.')
                return
        """
        if words[1] in target.attr:
            t = type(target[words[1]])
            try:
                v = t(words[2])
            except:
                ob.sendLine('☞ 잘못된 값입니다.')
                return
        else:
            try:
                v = int(words[2])
            except:
                v = words[2]
        target[words[1]] = v
        ob.sendLine('☞ 값이 설정되었습니다.')
        

