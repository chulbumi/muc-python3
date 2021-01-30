# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        words = line.split()
        if line == '':
            target = ob.env
        else:
            target = ob.env.findObjName(line)
        if target == None:
            target = ob.findObjName(line)
            if target == None:
                ob.sendLine('☞ 그런 대상이 없어요!')
                return
        if is_item(target):
            msg = '[아이템정보]\n\n'
        elif is_mob(target):
            msg = '[몹정보]\n\n'
        elif line == '':
            msg = '[맵정보]\r\n'
        else:
            ob.sendLine('☞ 저장할 수 없어요!')
            return
        l = list(target.attr.keys())
        l.sort()
        for at in l:
            msg += '#%s\n' % at
            for m in str(target.attr[at]):
                msg += ':%s\n' % m
            msg += '\n'

        try:
            f = open(target.path, 'w')
        except:
            ob.sendLine('* 파일 열기를 실패하였습니다.')
            return False
        f.write(msg)
        f.close()
       
        ob.sendLine('* %s 저장되었습니다.' % target.path)
        
