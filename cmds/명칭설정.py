# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if ob['직위'] != '방주':
            ob.sendLine('☞ 방파의 방주만이 할 수 있습니다.')
            return
        words = line.split()
        l = ['방주', '부방주', '장로', '방파인']
        if line == '' or len(words) < 2 or words[0] not in l:
            ob.sendLine('☞ 사용법 : [방주|부방주|장로|방파인] [이름] 명칭설정')
            return

        GUILD[ob['소속']]['%s명칭' % words[0]] = words[1]
        GUILD.save()
        print(GUILD[ob['소속']]['%s명칭' % words[0]])
        msg = '%s %s의 명칭을 [1m%s[0;37m%s 변경하여 선포합니다.' % (ob.han_iga(), words[0], words[1], han_uro(words[1]))
        ob.sendGroup(msg, prompt = True)
        
